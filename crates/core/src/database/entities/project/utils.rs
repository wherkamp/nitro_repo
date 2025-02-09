use derive_builder::Builder;
use serde::Serialize;
use sqlx::{PgPool, prelude::FromRow};
use tracing::debug;
use uuid::Uuid;

use crate::database::{entities::project::DBProjectColumn, tools::TableQuery};

use super::{DBProject, ProjectDBType};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, FromRow)]
pub struct ProjectLookupResult {
    pub id: Uuid,
    pub scope: Option<String>,
    pub project_key: String,
    pub name: String,
    pub project_path: String,
}
impl TableQuery for ProjectLookupResult {
    type Table = DBProject;

    fn columns() -> Vec<<Self::Table as crate::database::prelude::TableType>::Columns>
    where
        Self: Sized,
    {
        vec![
            DBProjectColumn::Id,
            DBProjectColumn::Scope,
            DBProjectColumn::ProjectKey,
            DBProjectColumn::Name,
            DBProjectColumn::ProjectPath,
        ]
    }
}
impl ProjectDBType for ProjectLookupResult {}
#[derive(Debug, Clone, PartialEq, Eq, Builder)]
pub struct ProjectLookup {
    pub project_key: Option<ProjectkeyLookup>,
    pub scope: Option<String>,
    pub name: Option<String>,
    pub storage_path: Option<String>,
    pub repository: Uuid,
}
impl ProjectLookup {
    pub async fn execute(self, database: &PgPool) -> Result<Vec<ProjectLookupResult>, sqlx::Error> {
        let Self {
            project_key,
            scope,
            name,
            repository,
            storage_path,
        } = self;

        let mut where_values = vec![];
        if let Some(project_key) = project_key {
            where_values.push(project_key.query_string("project_key", where_values.len()));
        }
        if let Some(scope) = scope {
            where_values.push((
                format!("LOWER(scope) = {}", where_values.len() + 1),
                scope.to_lowercase(),
            ));
        }
        if let Some(name) = name {
            where_values.push((
                format!("LOWER(name) = {}", where_values.len() + 1),
                name.to_lowercase(),
            ));
        }
        if let Some(storage_path) = storage_path {
            where_values.push((
                format!("LOWER(storage_path) = {}", where_values.len() + 1),
                storage_path.to_lowercase(),
            ));
        }
        let where_clause = if where_values.is_empty() {
            "".to_string()
        } else {
            where_values
                .iter()
                .map(|(x, _)| x.as_str())
                .collect::<Vec<&str>>()
                .join(" OR ")
        };
        let query = format!(
            r#"SELECT id, scope, project_key, name, storage_path  FROM projects WHERE repository = $1 AND ({})"#,
            where_clause
        );
        debug!(
            "Executing query: {} with parameters {}",
            query,
            where_values
                .iter()
                .map(|(_, value)| value.as_str())
                .collect::<Vec<&str>>()
                .join(", ")
        );
        let mut projects = sqlx::query_as::<_, ProjectLookupResult>(&query).bind(repository);
        for (_, value) in where_values {
            projects = projects.bind(value);
        }
        let projects = projects.fetch_all(database).await?;
        Ok(projects)
    }

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectkeyLookup {
    Full { value: String, ignore_case: bool },
    StartsWith { value: String, ignore_case: bool },
    EndsWith { value: String, ignore_case: bool },
    Contains { value: String, ignore_case: bool },
}
impl ProjectkeyLookup {
    pub fn query_string(self, column: &str, index: usize) -> (String, String) {
        match self {
            ProjectkeyLookup::Full { value, ignore_case } => {
                if ignore_case {
                    (
                        format!("LOWER({column}) = ${}", index + 1),
                        value.to_lowercase(),
                    )
                } else {
                    (format!("{column} = ${}", index + 1), value)
                }
            }
            ProjectkeyLookup::StartsWith { value, ignore_case } => {
                if ignore_case {
                    (format!("{column} ILIKE ${}%", index + 1), value)
                } else {
                    (format!("{column} LIKE ${}%", index + 1), value)
                }
            }
            ProjectkeyLookup::EndsWith { value, ignore_case } => {
                if ignore_case {
                    (format!("{column} ILIKE %${}", index + 1), value)
                } else {
                    (format!("{column} LIKE %${}", index + 1), value)
                }
            }

            ProjectkeyLookup::Contains { value, ignore_case } => {
                if ignore_case {
                    (format!("{column} ILIKE %${}%", index + 1), value)
                } else {
                    (format!("{column} LIKE %${}%", index + 1), value)
                }
            }
        }
    }
}

pub async fn does_project_id_exist(id: Uuid, database: &PgPool) -> Result<bool, sqlx::Error> {
    let project = sqlx::query("SELECT id FROM projects WHERE id = $1")
        .bind(id)
        .fetch_optional(database)
        .await?;
    Ok(project.is_some())
}
