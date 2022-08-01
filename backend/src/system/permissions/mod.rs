use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::repository::settings::Visibility;
use crate::repository::settings::{Policy, RepositoryConfig};
use crate::system::permissions::PermissionError::{RepositoryClassifier, StorageClassifier};

pub mod options;
pub mod orm;

#[derive(Error, Debug)]
pub enum PermissionError {
    #[error("Unable to Parse Repository String {0}")]
    ParseError(String),

    #[error("Unable to Parse Storage String")]
    StorageClassifier,
    #[error("Unable to Parse Repository String")]
    RepositoryClassifier,
    #[error("Unable to Parse Repository String {0}")]
    RepositoryClassifierParseError(serde_json::Error),
}

impl From<serde_json::Error> for PermissionError {
    fn from(error: serde_json::Error) -> Self {
        PermissionError::RepositoryClassifierParseError(error)
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone, Default)]
pub struct UserPermissions {
    pub disabled: bool,
    pub admin: bool,
    pub user_manager: bool,
    pub repository_manager: bool,
    #[serde(default)]
    pub deployer: RepositoryPermission,
    #[serde(default)]
    pub viewer: RepositoryPermission,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct RepositoryPermission {
    pub permissions: Vec<String>,
}
impl Default for RepositoryPermission {
    fn default() -> Self {
        RepositoryPermission {
            permissions: vec!["*".to_string()],
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RepositoryPermissionValue {
    pub policy: Option<Policy>,
    #[serde(rename = "type")]
    pub repo_type: Option<String>,
}

pub fn can_deploy(
    user_perms: &UserPermissions,
    repo: &RepositoryConfig,
) -> Result<bool, PermissionError> {
    if user_perms.disabled {
        return Ok(false);
    }
    if user_perms.admin {
        return Ok(true);
    }

    if !user_perms.deployer.permissions.is_empty() {
        return can(repo, &user_perms.deployer);
    }
    Ok(false)
}

pub fn can_read(
    user_perms: &UserPermissions,
    repo: &RepositoryConfig,
) -> Result<bool, PermissionError> {
    if user_perms.disabled {
        return Ok(false);
    }
    if user_perms.admin {
        return Ok(true);
    }

    match repo.visibility {
        Visibility::Public => Ok(true),
        Visibility::Private => {
            if !user_perms.viewer.permissions.is_empty() && can(repo, &user_perms.viewer)? {
                return Ok(true);
            }
            can_deploy(user_perms, repo)
        }
        Visibility::Hidden => Ok(true),
    }
}

pub fn can(repo: &RepositoryConfig, perms: &RepositoryPermission) -> Result<bool, PermissionError> {
    if perms.permissions.is_empty() {
        // If nothing is set. It is a all view type of scenario
        return Ok(true);
    }
    let repository = repo.name.clone();
    let storage = repo.storage.clone();
    for perm_string in perms.permissions.iter() {
        let split = perm_string.split('/').collect::<Vec<&str>>();
        let storage_perm = split.first().ok_or(StorageClassifier)?.to_string();
        if !storage_perm.eq("*") && !storage_perm.eq_ignore_ascii_case(&storage) {
            continue;
        }
        drop(storage_perm);
        let repository_perm = split.get(1).ok_or(RepositoryClassifier)?.to_string();
        if repository_perm.eq("*") || repository_perm.eq(&repository) {
            return Ok(true);
        }
        if repository_perm.starts_with('{') && repository_perm.ends_with('}') {
            let permission: RepositoryPermissionValue = serde_json::from_str(&repository_perm)?;
            if let Some(policy) = &permission.policy {
                if !policy.eq(&repo.policy) {
                    return Ok(false);
                }
            }
            if let Some(repo_type) = &permission.repo_type {
                if !repo_type.eq(&repo.repository_type.to_string()) {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
    }
    Ok(false)
}
