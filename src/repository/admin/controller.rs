use std::fs::create_dir_all;
use std::path::PathBuf;
use std::str::FromStr;

use actix_web::{get, HttpRequest, post, web};
use serde::{Deserialize, Serialize};

use crate::api_response::{APIResponse, SiteResponse};
use crate::DbPool;
use crate::error::response::{already_exists, bad_request, not_found, unauthorized};
use crate::repository::action::{add_new_repository, get_repo_by_id, get_repo_by_name_and_storage, get_repositories, update_deploy_settings, update_repo};
use crate::repository::models::{ReportGeneration, Repository, RepositoryListResponse, RepositorySettings, SecurityRules, UpdateFrontend, UpdateSettings, Visibility, Webhook};
use crate::storage::action::get_storage_by_name;
use crate::system::action::get_user_by_username;
use crate::system::utils::get_user_by_header;
use crate::utils::get_current_time;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRepositories {
    pub repositories: Vec<RepositoryListResponse>,
}

#[get("/api/repositories/list")]
pub async fn list_repos(pool: web::Data<DbPool>, r: HttpRequest) -> SiteResponse {
    let connection = pool.get()?;

    let user = get_user_by_header(r.headers(), &connection)?;
    if user.is_none() || !user.unwrap().permissions.admin {
        return unauthorized();
    }
    let vec = get_repositories(&connection)?;

    let response = ListRepositories { repositories: vec };
    APIResponse::new(true, Some(response)).respond(&r)
}

#[get("/api/repositories/get/{repo}")]
pub async fn get_repo(
    pool: web::Data<DbPool>,
    r: HttpRequest,
    path: web::Path<i64>,
) -> SiteResponse {
    let connection = pool.get()?;

    let user = get_user_by_header(r.headers(), &connection)?;
    if user.is_none() || !user.unwrap().permissions.admin {
        return unauthorized();
    }
    let repo = get_repo_by_id(&path.into_inner(), &connection)?;

    APIResponse::respond_new(repo, &r)
}

#[get("/api/repositories/get/{storage}/{repo}")]
pub async fn get_repo_deployer(
    pool: web::Data<DbPool>,
    r: HttpRequest,
    path: web::Path<(String, String)>,
) -> SiteResponse {
    let (storage, repo) = path.into_inner();
    let connection = pool.get()?;

    let user = get_user_by_header(r.headers(), &connection)?;
    if user.is_none() || !user.unwrap().permissions.deployer {
        return unauthorized();
    }
    let storage = get_storage_by_name(&storage, &connection)?;
    if storage.is_none() {
        return not_found();
    }
    let repo = get_repo_by_name_and_storage(&repo, &storage.unwrap().id, &connection)?;

    APIResponse::respond_new(repo, &r)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NewRepo {
    pub name: String,
    pub storage: String,
    pub repo: String,
    pub settings: RepositorySettings,
}

#[post("/api/admin/repository/add")]
pub async fn add_repo(
    pool: web::Data<DbPool>,
    r: HttpRequest,
    nc: web::Json<NewRepo>,
) -> SiteResponse {
    let connection = pool.get()?;

    let user = get_user_by_header(r.headers(), &connection)?;
    if user.is_none() || !user.unwrap().permissions.admin {
        return unauthorized();
    }
    let storage = crate::storage::action::get_storage_by_name(&nc.storage, &connection)?;
    if storage.is_none() {
        return not_found();
    }
    let storage = storage.unwrap();

    let option = get_repo_by_name_and_storage(&nc.name, &storage.id, &connection)?;
    if option.is_some() {
        return already_exists();
    }
    let repository = Repository {
        id: 0,

        name: nc.0.name,
        repo_type: nc.0.repo,
        storage: storage.id,
        settings: nc.0.settings,
        security: SecurityRules {
            deployers: vec![],
            visibility: Visibility::Public,
            readers: vec![],
        },
        deploy_settings: Default::default(),
        created: get_current_time(),
    };
    add_new_repository(&repository, &connection)?;
    let buf = PathBuf::new()
        .join("storages")
        .join(&storage.name)
        .join(&repository.name);
    if !buf.exists() {
        create_dir_all(buf)?;
    }
    let option = get_repo_by_name_and_storage(&repository.name, &storage.id, &connection)?;

    APIResponse::from(option).respond(&r)
}

#[post("/api/admin/repository/{storage}/{repo}/modify/settings/general")]
pub async fn modify_general_settings(
    pool: web::Data<DbPool>,
    r: HttpRequest,
    path: web::Path<(String, String)>,
    nc: web::Json<UpdateSettings>,
) -> SiteResponse {
    let (storage, repo) = path.into_inner();

    let connection = pool.get()?;

    let user = get_user_by_header(r.headers(), &connection)?;
    if user.is_none() || !user.unwrap().permissions.admin {
        return unauthorized();
    }
    let storage = get_storage_by_name(&storage, &connection)?;
    if storage.is_none() {
        return not_found();
    }
    let storage = storage.unwrap();
    let repository = get_repo_by_name_and_storage(&repo, &storage.id, &connection)?;
    if repository.is_none() {
        return not_found();
    }
    let mut repository = repository.unwrap();
    repository.settings.update_general(nc.0);
    update_repo(&repository, &connection)?;
    APIResponse::new(true, Some(repository)).respond(&r)
}

#[post("/api/admin/repository/{storage}/{repo}/modify/settings/frontend")]
pub async fn modify_frontend_settings(
    pool: web::Data<DbPool>,
    r: HttpRequest,
    path: web::Path<(String, String)>,
    nc: web::Json<UpdateFrontend>,
) -> SiteResponse {
    let (storage, repo) = path.into_inner();

    let connection = pool.get()?;

    let user = get_user_by_header(r.headers(), &connection)?;
    if user.is_none() || !user.unwrap().permissions.admin {
        return unauthorized();
    }
    let storage = get_storage_by_name(&storage, &connection)?;
    if storage.is_none() {
        return not_found();
    }
    let storage = storage.unwrap();
    let repository = get_repo_by_name_and_storage(&repo, &storage.id, &connection)?;
    if repository.is_none() {
        return not_found();
    }
    let mut repository = repository.unwrap();
    repository.settings.update_frontend(nc.0);
    update_repo(&repository, &connection)?;
    APIResponse::new(true, Some(repository)).respond(&r)
}

#[post("/api/admin/repository/{storage}/{repo}/modify/security/visibility/{visibility}")]
pub async fn modify_security(
    pool: web::Data<DbPool>,
    r: HttpRequest,
    path: web::Path<(String, String, String)>,
) -> SiteResponse {
    let (storage, repo, visibility) = path.into_inner();

    let connection = pool.get()?;

    let user = get_user_by_header(r.headers(), &connection)?;
    if user.is_none() || !user.unwrap().permissions.admin {
        return unauthorized();
    }
    let storage = get_storage_by_name(&storage, &connection)?;
    if storage.is_none() {
        return not_found();
    }
    let storage = storage.unwrap();
    let repository = get_repo_by_name_and_storage(&repo, &storage.id, &connection)?;
    if repository.is_none() {
        return not_found();
    }
    let mut repository = repository.unwrap();
    //TODO BAD CODE
    let visibility = Visibility::from_str(visibility.as_str()).unwrap();
    repository.security.set_visibility(visibility);
    update_repo(&repository, &connection)?;
    APIResponse::new(true, Some(repository)).respond(&r)
}

#[post("/api/admin/repository/{storage}/{repo}/modify/security/{what}/{action}/{user}")]
pub async fn update_deployers_readers(
    pool: web::Data<DbPool>,
    r: HttpRequest,
    path: web::Path<(String, String, String, String, String)>,
) -> SiteResponse {
    let (storage, repo,what,action,user) = path.into_inner();

    let connection = pool.get()?;

    let admin = get_user_by_header(r.headers(), &connection)?;
    if admin.is_none() || !admin.unwrap().permissions.admin {
        return unauthorized();
    }
    let storage = get_storage_by_name(&storage, &connection)?;
    if storage.is_none() {
        return not_found();
    }
    let storage = storage.unwrap();
    let repository = get_repo_by_name_and_storage(&repo, &storage.id, &connection)?;
    if repository.is_none() {
        return not_found();
    }
    let mut repository = repository.unwrap();
    let user = get_user_by_username(&user, &connection)?;
    if user.is_none() {
        return not_found();
    }
    let user = user.unwrap();
    match action.as_str(){
        "deployers" => match what.as_str() {
            "add" => {
                repository.security.deployers.push(user.id);
            }
            "remove" => {
                let filter = repository
                    .security
                    .deployers
                    .iter()
                    .position(|x| x == &user.id);
                if filter.is_some() {
                    repository.security.deployers.remove(filter.unwrap());
                }
            }
            _ => return bad_request("Must be Add or Remove"),
        },
        "readers" => match what.as_str() {
            "add" => {
                repository.security.readers.push(user.id);
            }
            "remove" => {
                let filter = repository
                    .security
                    .readers
                    .iter()
                    .position(|x| x == &user.id);
                if filter.is_some() {
                    repository.security.readers.remove(filter.unwrap());
                }
            }
            _ => return bad_request("Must be Add or Remove"),
        },
        _ => return bad_request("Must be Deployers or Readers"),
    }
    update_repo(&repository, &connection)?;
    APIResponse::new(true, Some(repository)).respond(&r)
}
#[post("/api/admin/repository/{storage}/{repository}/modify/deploy/report")]
pub async fn modify_deploy(
    pool: web::Data<DbPool>,
    r: HttpRequest,
    path: web::Path<(String, String)>,
    nc: web::Json<ReportGeneration>,
) -> SiteResponse {
    let (storage, repository) = path.into_inner();

    let connection = pool.get()?;

    let user = get_user_by_header(r.headers(), &connection)?;
    if user.is_none() || !user.unwrap().permissions.admin {
        return unauthorized();
    }
    let storage = get_storage_by_name(&storage, &connection)?;
    if storage.is_none() {
        return not_found();
    }
    let storage = storage.unwrap();
    let repository_value = get_repo_by_name_and_storage(&repository, &storage.id, &connection)?;
    if repository_value.is_none() {
        return not_found();
    }
    let repo = repository_value.unwrap();
    let mut deploy_settings = repo.deploy_settings;
    deploy_settings.report_generation = nc.0;
    update_deploy_settings(&repo.id, &deploy_settings, &connection)?;

    APIResponse::respond_new(get_repo_by_name_and_storage(&repository, &storage.id, &connection)?, &r)
}

#[post("/api/admin/repository/{storage}/{repo}/modify/deploy/webhook/add")]
pub async fn add_webhook(
    pool: web::Data<DbPool>,
    r: HttpRequest,
    path: web::Path<(String, String)>,
    nc: web::Json<Webhook>,
) -> SiteResponse {
    let (storage, repo) = path.into_inner();

    let connection = pool.get()?;

    let user = get_user_by_header(r.headers(), &connection)?;
    if user.is_none() || !user.unwrap().permissions.admin {
        return unauthorized();
    }
    let storage = get_storage_by_name(&storage, &connection)?;
    if storage.is_none() {
        return not_found();
    }
    let storage = storage.unwrap();
    let repository_value = get_repo_by_name_and_storage(&repo, &storage.id, &connection)?;
    if repository_value.is_none() {
        return not_found();
    }
    let repository_value = repository_value.unwrap();
    let mut deploy_settings = repository_value.deploy_settings;
    deploy_settings.add_webhook(nc.0);
    update_deploy_settings(&repository_value.id, &deploy_settings, &connection)?;
    APIResponse::respond_new(get_repo_by_name_and_storage(&repo, &storage.id, &connection)?, &r)
}

#[post("/api/admin/repository/{storage}/{repo}/modify/deploy/webhook/add/{webhook}")]
pub async fn remove_webhook(
    pool: web::Data<DbPool>,
    r: HttpRequest,
    path: web::Path<(String, String, String)>,
) -> SiteResponse {
    let (storage, repo,webhook) = path.into_inner();

    let connection = pool.get()?;
    let user = get_user_by_header(r.headers(), &connection)?;
    if user.is_none() || !user.unwrap().permissions.admin {
        return unauthorized();
    }
    let storage = get_storage_by_name(&storage,  &connection)?;
    if storage.is_none() {
        return not_found();
    }
    let storage = storage.unwrap();
    let repository = get_repo_by_name_and_storage(&repo, &storage.id, &connection)?;
    if repository.is_none() {
        return not_found();
    }
    let repository = repository.unwrap();
    let mut deploy_settings = repository.deploy_settings;
    deploy_settings.remove_hook(webhook);
    APIResponse::respond_new(get_repo_by_name_and_storage(&repo, &storage.id, &connection)?, &r)
}

