use diesel::prelude::*;
use diesel::MysqlConnection;

use crate::repository;
use crate::repository::models::{DeploySettings, Repository, RepositoryListResponse};
use crate::repository::models::{RepositorySettings, SecurityRules};

pub fn update_repo(repo: &Repository, conn: &MysqlConnection) -> Result<(), diesel::result::Error> {
    use crate::schema::repositories::dsl::*;
    let _result1 = diesel::update(repositories.filter(id.eq(repo.id)))
        .set((
            settings.eq(repo.settings.clone()),
            security.eq(repo.security.clone()),
        ))
        .execute(conn);
    Ok(())
}

pub fn update_deploy_settings(
    repo: &i64,
    deploy: &DeploySettings,
    conn: &MysqlConnection,
) -> Result<(), diesel::result::Error> {
    use crate::schema::repositories::dsl::*;
    let _result1 = diesel::update(repositories.filter(id.eq(repo)))
        .set((deploy_settings.eq(deploy),))
        .execute(conn);
    Ok(())
}

pub fn update_repo_settings(
    repo: &i64,
    s: &RepositorySettings,
    conn: &MysqlConnection,
) -> Result<(), diesel::result::Error> {
    use crate::schema::repositories::dsl::*;
    let _result1 = diesel::update(repositories.filter(id.eq(repo)))
        .set((settings.eq(s),))
        .execute(conn)?;
    Ok(())
}

pub fn update_repo_security(
    repo: &i64,
    rules: &SecurityRules,
    conn: &MysqlConnection,
) -> Result<(), diesel::result::Error> {
    use crate::schema::repositories::dsl::*;
    let _result1 = diesel::update(repositories.filter(id.eq(repo)))
        .set((security.eq(rules),))
        .execute(conn)?;
    Ok(())
}

pub fn get_repo_by_name_and_storage(
    repo: &str,
    sto: &str,
    conn: &MysqlConnection,
) -> Result<Option<repository::models::Repository>, diesel::result::Error> {
    use crate::schema::repositories::dsl::*;

    let found_mod = repositories
        .filter(name.like(repo).and(storage.eq(sto)))
        .first::<repository::models::Repository>(conn)
        .optional()?;

    Ok(found_mod)
}
pub fn get_repo_by_id(
    repo: &i64,
    conn: &MysqlConnection,
) -> Result<Option<repository::models::Repository>, diesel::result::Error> {
    use crate::schema::repositories::dsl::*;

    let found_mod = repositories
        .filter(id.eq(repo))
        .first::<repository::models::Repository>(conn)
        .optional()?;

    Ok(found_mod)
}

pub fn add_new_repository(
    s: &Repository,
    conn: &MysqlConnection,
) -> Result<(), diesel::result::Error> {
    use crate::schema::repositories::dsl::*;
    diesel::insert_into(repositories)
        .values(s)
        .execute(conn)
        .unwrap();
    Ok(())
}

pub fn get_repositories(
    conn: &MysqlConnection,
) -> Result<Vec<RepositoryListResponse>, diesel::result::Error> {
    use crate::schema::repositories::dsl::*;
    repositories
        .select((id, name, repo_type, storage))
        .load::<RepositoryListResponse>(conn)
}

pub fn get_repositories_by_storage(
    stor: &str,
    conn: &MysqlConnection,
) -> Result<Vec<repository::models::Repository>, diesel::result::Error> {
    use crate::schema::repositories::dsl::*;
    repositories
        .filter(storage.eq(stor))
        .load::<repository::models::Repository>(conn)
}
