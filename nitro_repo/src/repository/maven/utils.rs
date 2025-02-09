use std::fmt::Debug;

use maven_rs::pom::Pom;
use nr_core::{
    database::entities::project::{
        DBProject, NewProject, NewProjectBuilder, NewProjectMember, ProjectDBType,
        update::UpdateProjectVersion, versions::DBProjectVersion, versions::NewVersion,
        versions::NewVersionBuilder,
    },
    repository::project::{ReleaseType, VersionData, VersionDataBuilder},
    storage::{FileTypeCheck, StoragePath},
    user::permissions::{HasPermissions, RepositoryActions},
};

use nr_storage::Storage;
use tracing::{Level, error, event, info, instrument, trace};
use uuid::Uuid;

use super::{MavenError, RepoResponse, RepositoryAuthentication, RepositoryHandlerError};
use crate::{error::BadRequestErrors, repository::Repository};

/// Utilities for Maven Repositories
pub trait MavenRepositoryExt: Repository + Debug {
    /// Checks if the user has the correct permissions to read the repository
    ///
    /// If not authenticated at all, it will return a `WWW-Authenticate` header with the `Basic` scheme (Maven gets confused otherwise)
    async fn check_read(
        &self,
        authentication: &RepositoryAuthentication,
    ) -> Result<Option<RepoResponse>, RepositoryHandlerError> {
        if self.visibility().is_private() {
            if authentication.is_no_identification() {
                return Ok(Some(RepoResponse::www_authenticate("Basic")));
            } else if !(authentication
                .has_action(RepositoryActions::Read, self.id(), self.site().as_ref())
                .await?)
            {
                return Ok(Some(RepoResponse::forbidden()));
            }
        }
        Ok(None)
    }
    /// Checks if the user has the correct permissions to read the repository
    /// If the repository is hidden and the requested file is a directory, then the user must have the read permission
    async fn indexing_check<T: FileTypeCheck>(
        &self,
        file_response: T,
        authentication: &RepositoryAuthentication,
    ) -> Result<RepoResponse, MavenError>
    where
        RepoResponse: From<T>,
    {
        let visibility = self.visibility();
        if file_response.is_directory()
            && visibility.is_hidden()
            && !authentication
                .has_action(RepositoryActions::Read, self.id(), self.site().as_ref())
                .await?
        {
            return Ok(RepoResponse::indexing_not_allowed());
        }
        // File is not a directory, so we can check the permissions
        Ok(RepoResponse::from(file_response))
    }

    /// Same as [indexing_check] but for an Option
    async fn indexing_check_option<T: FileTypeCheck>(
        &self,
        file_response: Option<T>,
        authentication: &RepositoryAuthentication,
    ) -> Result<RepoResponse, MavenError>
    where
        RepoResponse: From<T>,
        RepoResponse: From<Option<T>>,
    {
        if let Some(file_response) = file_response {
            self.indexing_check(file_response, authentication).await
        } else {
            Ok(RepoResponse::from(None))
        }
    }
    #[instrument(name = "MavenRepository::parse_pom")]
    fn parse_pom(&self, pom: Vec<u8>) -> Result<Pom, MavenError> {
        let pom_file = String::from_utf8(pom).map_err(BadRequestErrors::from)?;
        trace!(?pom_file, "Parsing POM file");
        let pom: maven_rs::pom::Pom = maven_rs::quick_xml::de::from_str(&pom_file)?;
        Ok(pom)
    }
    #[instrument]
    async fn post_pom_upload_inner(
        &self,
        pom_directory: StoragePath,
        publisher: Option<i32>,
        pom: Pom,
    ) -> Result<(), MavenError> {
        let group_id = pom
            .get_group_id()
            .ok_or(MavenError::MissingFromPom("groupId"))?;
        let project_key = format!("{}:{}", group_id, pom.artifact_id);
        let version_directory = pom_directory.clone().parent();
        let db_project =
            DBProject::find_by_project_key(&project_key, self.id(), &self.site().database).await?;
        let (project_id, project_dir) = if let Some(project) = db_project {
            (project.id, StoragePath::from(project.project_path))
        } else {
            let project_directory = version_directory.clone().parent();
            trace!(?project_directory, "Creating Project");
            let project = pom_to_db_project(project_directory.clone(), self.id(), pom.clone())?;
            let project = project.insert(&self.site().database).await?;
            if let Some(publisher) = publisher {
                let new_member = NewProjectMember::new_owner(publisher, project.id);
                new_member.insert_no_return(&self.site().database).await?;
            } else {
                info!(?db_project, "No publisher provided for project");
            }
            info!(?project, "Created Project");
            (project.id, project_directory)
        };
        let mut repository_meta = self
            .get_storage()
            .get_repository_meta(self.id(), &project_dir)
            .await?
            .unwrap_or_default();
        repository_meta.set_project_id(project_id);

        event!(
            Level::DEBUG,
            ?repository_meta,
            ?project_dir,
            "Setting Repository Meta"
        );
        self.get_storage()
            .put_repository_meta(self.id(), &project_dir, repository_meta)
            .await?;

        self.add_or_update_version(version_directory, project_id, publisher, pom)
            .await?;
        Ok(())
    }

    async fn post_pom_upload(&self, pom_directory: StoragePath, publisher: Option<i32>, pom: Pom) {
        match self
            .post_pom_upload_inner(pom_directory, publisher, pom)
            .await
        {
            Ok(()) => {}
            Err(e) => {
                error!(?e, "Failed to handle POM Upload");
            }
        }
    }
    async fn add_or_update_version(
        &self,
        version_directory: StoragePath,
        project_id: Uuid,
        publisher: Option<i32>,
        pom: Pom,
    ) -> Result<(), MavenError> {
        let version = pom
            .get_version()
            .ok_or(MavenError::MissingFromPom("version"))?;
        let db_version = DBProjectVersion::find_by_version_and_project(
            version,
            project_id,
            &self.site().database,
        )
        .await?;
        let version_id = if let Some(version) = db_version {
            let update = pom_to_update_db_project_version(pom)?;
            update.update(version.id, &self.site().database).await?;
            version
        } else {
            let version = pom_to_db_project_version(
                project_id,
                version_directory.clone(),
                publisher,
                pom.clone(),
            )?;
            let db_version = version.insert(&self.site().database).await?;
            info!(?db_version, "Created Version");
            db_version
        };

        let mut repository_meta = self
            .get_storage()
            .get_repository_meta(self.id(), &version_directory)
            .await?
            .unwrap_or_default();

        repository_meta.set_version_id(version_id.id);
        repository_meta.set_project_id(project_id);
        event!(
            Level::DEBUG,
            ?repository_meta,
            ?version_directory,
            "Setting Repository Meta"
        );
        self.get_storage()
            .put_repository_meta(self.id(), &version_directory, repository_meta)
            .await?;
        Ok(())
    }
}
pub fn pom_to_db_project(
    project_path: StoragePath,
    repository: Uuid,
    pom: Pom,
) -> Result<NewProject, MavenError> {
    let group_id = pom
        .get_group_id()
        .ok_or(MavenError::MissingFromPom("groupId"))?;
    let result = NewProjectBuilder::default()
        .project_key(format!("{}:{}", group_id, pom.artifact_id))
        .scope(Some(group_id.to_owned()))
        .name(pom.name.unwrap_or(pom.artifact_id))
        .description(pom.description)
        .repository(repository)
        .storage_path(project_path.to_string())
        .build()?;
    Ok(result)
}
pub fn pom_to_db_project_version(
    project_id: Uuid,
    version_path: StoragePath,
    publisher: Option<i32>,
    pom: Pom,
) -> Result<NewVersion, MavenError> {
    let version = pom
        .get_version()
        .ok_or(MavenError::MissingFromPom("version"))
        .map(|x| x.to_owned())?;
    let version_data = VersionDataBuilder::default()
        .description(pom.description)
        .build()?;

    let release_type = ReleaseType::release_type_from_version(&version);
    let result = NewVersionBuilder::default()
        .project_id(project_id)
        .version(version)
        .publisher(publisher)
        .version_path(version_path.to_string())
        .release_type(release_type)
        .extra(version_data)
        .build()?;
    Ok(result)
}

pub fn pom_to_update_db_project_version(pom: Pom) -> Result<UpdateProjectVersion, MavenError> {
    let release_type = pom
        .get_version()
        .ok_or(MavenError::MissingFromPom("version"))
        .map(ReleaseType::release_type_from_version)?;

    let extra = pom_to_version_extra_data(pom)?;
    let result = UpdateProjectVersion {
        release_type: Some(release_type),
        extra: Some(extra),
        ..Default::default()
    };
    Ok(result)
}
pub fn pom_to_version_extra_data(pom: Pom) -> Result<VersionData, MavenError> {
    let extra = VersionDataBuilder::default()
        .description(pom.description)
        .build()?;
    Ok(extra)
}
