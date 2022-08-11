use crate::repository::settings::{RepositoryConfig, RepositoryConfigHandler, RepositoryConfigType};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::error::internal_error::InternalError;
use crate::repository::handler::Repository;
use crate::storage::models::Storage;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq, JsonSchema)]
pub enum PageType {
    #[default]
    None,
    Markdown,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, JsonSchema)]
pub struct RepositoryPage {
    #[serde(default)]
    pub page_type: PageType,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct UpdateRepositoryPage {
    pub settings: RepositoryPage,
    pub page: Option<String>,
}

impl RepositoryConfigType for RepositoryPage {
    fn config_name() -> &'static str {
        "page.json"
    }
}


pub mod multi_web {
    use std::sync::Arc;
    use actix_web::{get, HttpResponse};
    use actix_web::web::{Data, Path};
    use log::error;
    use crate::generators::GeneratorCache;
    use crate::repository;
    use crate::repository::handler::{DynamicRepositoryHandler, Repository};
    use crate::repository::settings::repository_page::{RepositoryPage, UpdateRepositoryPage};
    use crate::repository::settings::RepositoryConfigHandler;
    use crate::storage::DynamicStorage;
    use crate::storage::models::Storage;

    pub async fn get_page(
        storage_handler: actix_web::web::Data<crate::storage::multi::MultiStorageController<crate::storage::DynamicStorage>>,
        database: actix_web::web::Data<sea_orm::DatabaseConnection>,
        auth: crate::authentication::Authentication,
        path_params: Path<(String, String)>,
    ) -> actix_web::Result<actix_web::HttpResponse> {
        use crate::storage::models::Storage;
        use crate::system::permissions::permissions_checker::CanIDo;
        let user = auth.get_user(&database).await??;
        user.can_i_edit_repos()?;
        let (storage_name, repository_name) = path_params.into_inner();
        let storage = crate::helpers::get_storage!( storage_handler, storage_name );
        let repository = crate::helpers::get_repository!( storage, repository_name );
        match repository.as_ref() {
            crate::repository::handler::DynamicRepositoryHandler::Maven(repository) => {
                let value = crate::repository::settings::RepositoryConfigHandler::<RepositoryPage>::get(repository).clone();
                let page = if let Some(data) = storage
                    .get_file(repository.get_repository(), ".config.nitro_repo/README.md")
                    .await? {
                    String::from_utf8(data).unwrap_or_default()
                } else {
                    String::new()
                };


                Ok(HttpResponse::Ok().json(UpdateRepositoryPage{
                    settings: value,
                    page: Some(page),
                }))
            }
            _ => {
                return Ok(actix_web::HttpResponse::BadRequest().body("Repository type not supported".to_string()));
            }
        }
    }

    pub async fn put_page(
        storage_handler: actix_web::web::Data<crate::storage::multi::MultiStorageController<crate::storage::DynamicStorage>>,
        database: actix_web::web::Data<sea_orm::DatabaseConnection>,
        auth: crate::authentication::Authentication,
        path_params: Path<(String, String)>,
        body: actix_web::web::Json<UpdateRepositoryPage>,
        generator: Data<GeneratorCache>,
    ) -> actix_web::Result<actix_web::HttpResponse> {
        use crate::storage::models::Storage;
        use crate::system::permissions::permissions_checker::CanIDo;
        let user = auth.get_user(&database).await??;
        user.can_i_edit_repos()?;
        let (storage_name, repository_name) = path_params.into_inner();
        let storage = crate::helpers::get_storage!( storage_handler, storage_name );
        let (name, mut repository) = crate::helpers::take_repository!( storage, repository_name );
        let body = body.into_inner();

        let result = match repository {
            DynamicRepositoryHandler::Maven(ref mut repository) => {
                let value = crate::repository::settings::RepositoryConfigHandler::<RepositoryPage>::update(repository, body.settings);
                if value.is_ok() {
                    save_config(generator, &storage, body.page, repository).await
                }
                value
            }
            _ => {
                return Ok(actix_web::HttpResponse::BadRequest().body("Repository type not supported".to_string()));
            }
        };
        storage.add_repository_for_updating(name, repository, false).await.expect("Failed to add repository for updating");
        result?;
        Ok(actix_web::HttpResponse::NoContent().finish())
    }

    async fn save_config<S: Storage, R>(generator: Data<GeneratorCache>, storage: &Arc<DynamicStorage>, body: Option<String>, repository: &mut R) where R: Repository<S> + RepositoryConfigHandler<RepositoryPage> {
        let config = RepositoryConfigHandler::<RepositoryPage>::get(repository);
        if let Err(error) = storage.save_repository_config(repository.get_repository(), config).await {
            error!("Failed to save repository config: {}", error);
        } else {
            return;
        }

        let page = body.unwrap_or_else(|| "".to_string());
        let cache_name = format!("{}/{}/.config.nitro_repo/README.html", repository.get_repository().storage, repository.get_repository().name);
        if let Err(error) = generator.remove_from_cache(&cache_name).await {
            error!("{}", error);
        }
        if let Err(error) = storage.save_file(repository.get_repository(), page.as_bytes(), ".config.nitro_repo/README.html").await {
            error!("{}", error);
        }
    }
    repository::web::multi::settings::define_init!(init,repository_page, get_page, put_page);
}
