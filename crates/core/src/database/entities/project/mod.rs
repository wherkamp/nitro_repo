use derive_builder::Builder;
use serde::Serialize;
use sqlx::{FromRow, PgPool, postgres::PgRow};
use tracing::instrument;
use utoipa::ToSchema;
use uuid::Uuid;
use versions::{DBProjectVersion, DBProjectVersionColumn};
mod new;
pub mod utils;
use crate::{database::prelude::*, repository::project::ReleaseType};
pub use new::*;
pub mod info;
pub mod update;
pub mod versions;
/// Implemented on different types of Project query result. Such as ProjectLookupResult
pub trait ProjectDBType: for<'r> FromRow<'r, PgRow> + Unpin + Send + Sync + TableQuery {
    async fn find_by_id(id: Uuid, database: &PgPool) -> Result<Option<Self>, sqlx::Error> {
        let columns = concat_columns(&Self::columns(), None);
        let project =
            sqlx::query_as::<_, Self>(&format!("SELECT {} FROM projects WHERE id = $1", columns))
                .bind(id)
                .fetch_optional(database)
                .await?;
        Ok(project)
    }
    async fn find_by_project_key(
        project_key: &str,
        repository: Uuid,
        database: &PgPool,
    ) -> Result<Option<Self>, sqlx::Error> {
        let columns = concat_columns(&Self::columns(), None);
        let project = sqlx::query_as::<_, Self>(&format!(
            "SELECT {} FROM projects WHERE repository_id = $1 AND LOWER(project_key) = $2",
            columns
        ))
        .bind(repository)
        .bind(project_key.to_lowercase())
        .fetch_optional(database)
        .await?;
        Ok(project)
    }
    #[instrument(skip(database))]
    async fn find_by_project_directory(
        directory: &str,
        repository: Uuid,
        database: &PgPool,
    ) -> Result<Option<Self>, sqlx::Error> {
        let columns = concat_columns(&Self::columns(), None);
        let project: Option<Self> = sqlx::query_as::<_, Self>(&format!(
            "SELECT {} FROM projects WHERE repository_id = $1 AND LOWER(storage_path) = $2",
            columns
        ))
        .bind(repository)
        .bind(directory.to_lowercase())
        .fetch_optional(database)
        .await?;
        Ok(project)
    }
    #[instrument(skip(database))]
    async fn get_by_id(id: Uuid, database: &PgPool) -> Result<Option<Self>, sqlx::Error> {
        let columns = concat_columns(&Self::columns(), None);

        let project =
            sqlx::query_as::<_, Self>(&format!("SELECT {} FROM projects WHERE id = $1", columns))
                .bind(id)
                .fetch_optional(database)
                .await?;
        Ok(project)
    }
    /// Finds a Project by the directory of the version
    async fn find_by_version_directory(
        directory: &str,
        repository: Uuid,
        database: &PgPool,
    ) -> Result<Option<Self>, sqlx::Error> {
        let columns = concat_columns(&Self::columns(), Some("P"));
        let project = sqlx::query_as::<_, Self>(
            &format!(
                "SELECT {} FROM projects as P FULL JOIN project_versions as V ON LOWER(V.release_path) = $1 AND V.project_id = P.id WHERE P.repository_id = $2",
                columns
            ),
        )
        .bind(directory.to_lowercase())
        .bind(repository)
        .fetch_optional(database)
        .await?;
        Ok(project)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, FromRow, ToSchema, Builder, Columns)]
pub struct DBProject {
    pub id: Uuid,
    /// Maven will use the groupId
    /// Cargo will be None
    /// NPM will use scope if it's set
    pub scope: Option<String>,
    /// Maven will use something like `{groupId}:{artifactId}`
    /// Cargo will use the `name` field
    ///
    /// This field is unique per repository
    pub project_key: String,
    /// Name of the project
    ///
    /// Maven will use the artifactId
    /// Cargo will use the `name` field
    /// NPM will use the `name` field
    pub name: String,
    /// A short description of the project
    pub description: Option<String>,
    /// The repository it belongs to
    pub repository_id: Uuid,
    /// The path to the project in the repository
    pub project_path: String,
    /// Last time the project was updated. This is updated when a new version is added
    pub updated_at: chrono::DateTime<chrono::FixedOffset>,
    /// When the project was created
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
}
impl TableType for DBProject {
    type Columns = DBProjectColumn;

    fn table_name() -> &'static str
    where
        Self: Sized,
    {
        "projects"
    }
}
impl ProjectDBType for DBProject {}
/// On the first push. The pusher will be added as a project member with write and manage permissions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, FromRow, ToSchema)]
pub struct DBProjectMember {
    pub id: i32,
    pub project_id: Uuid,
    pub user_id: i32,
    pub can_write: bool,
    pub can_manage: bool,
    pub added: chrono::DateTime<chrono::FixedOffset>,
}
impl DBProjectMember {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, FromRow)]
pub struct ProjectIds {
    pub project_id: Uuid,
    pub version_id: i32,
}

pub async fn latest_version(
    project_id: Uuid,
    release_type: ReleaseType,
    database: &PgPool,
) -> Result<Option<i32>, sqlx::Error> {
    let version_id: Option<i32> = SimpleSelectQueryBuilder::new(
        DBProjectVersion::table_name(),
        vec![DBProjectVersionColumn::Id],
    )
    .where_equals(DBProjectVersionColumn::ProjectId, project_id)
    .where_equals(DBProjectVersionColumn::ReleaseType, release_type)
    .order_by(DBProjectVersionColumn::CreatedAt, SQLOrder::Descending)
    .limit(1)
    .query_scalar()
    .fetch_optional(database)
    .await?;
    Ok(version_id)
}
