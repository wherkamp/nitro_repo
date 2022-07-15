use std::fmt;
use std::fmt::{Debug, Display, Formatter};

use actix_web::http::StatusCode;
use actix_web::ResponseError;
use serde_json::json;

use crate::error::internal_error::InternalError;
use crate::repository::settings::RepositoryConfig;
use crate::repository::settings::Visibility;
use crate::system::permissions::{can_deploy, can_read};
use crate::system::user::UserModel;

pub struct MissingPermission(String);

impl Debug for MissingPermission {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Forbidden: Missing Permission {}", &self.0)
    }
}

impl Display for MissingPermission {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let result = serde_json::to_string(&json!({
            "error": format!("Missing Permission {}", &self.0),
        }))
        .map_err(|_| fmt::Error)?;
        write!(f, "{}", result)
    }
}

impl ResponseError for MissingPermission {
    fn status_code(&self) -> StatusCode {
        StatusCode::FORBIDDEN
    }
}
impl From<&str> for MissingPermission {
    fn from(value: &str) -> Self {
        MissingPermission(format!("Missing Permission `{}`", value))
    }
}

pub trait CanIDo {
    fn can_i_edit_repos(&self) -> Result<(), MissingPermission>;
    fn can_i_edit_users(&self) -> Result<(), MissingPermission>;
    fn can_i_admin(&self) -> Result<(), MissingPermission>;
    fn can_deploy_to(
        &self,
        repo: &RepositoryConfig,
    ) -> Result<Option<MissingPermission>, InternalError>;
    fn can_read_from(
        &self,
        repo: &RepositoryConfig,
    ) -> Result<Option<MissingPermission>, InternalError>;
}
impl<E: CanIDo> CanIDo for Option<E> {
    fn can_i_edit_repos(&self) -> Result<(), MissingPermission> {
        match self.as_ref() {
            Some(e) => e.can_i_edit_repos(),
            None => Err(MissingPermission("can_i_edit_repos".to_string())),
        }
    }

    fn can_i_edit_users(&self) -> Result<(), MissingPermission> {
        match self.as_ref() {
            Some(e) => e.can_i_edit_users(),
            None => Err(MissingPermission("edit_users".to_string())),
        }
    }

    fn can_i_admin(&self) -> Result<(), MissingPermission> {
        match self.as_ref() {
            Some(e) => e.can_i_edit_users(),
            None => Err(MissingPermission("admin".to_string())),
        }
    }

    fn can_deploy_to(
        &self,
        repo: &RepositoryConfig,
    ) -> Result<Option<MissingPermission>, InternalError> {
        match self.as_ref() {
            Some(e) => e.can_deploy_to(repo),
            None => Ok(Some(MissingPermission("Logged In".to_string()))),
        }
    }

    fn can_read_from(
        &self,
        repo: &RepositoryConfig,
    ) -> Result<Option<MissingPermission>, InternalError> {
        match self.as_ref() {
            Some(e) => e.can_read_from(repo),
            None => Ok(Some(MissingPermission("Logged In".to_string()))),
        }
    }
}
impl CanIDo for UserModel {
    fn can_i_edit_repos(&self) -> Result<(), MissingPermission> {
        let permissions = &self.permissions;

        if !permissions.admin && !permissions.repository_manager {
            return Err("repository_manager".into());
        }
        Ok(())
    }

    fn can_i_edit_users(&self) -> Result<(), MissingPermission> {
        let permissions = &self.permissions;
        if !permissions.admin && !permissions.user_manager {
            return Err("user_manager".into());
        }
        Ok(())
    }

    fn can_i_admin(&self) -> Result<(), MissingPermission> {
        let permissions = &self.permissions;

        if !permissions.admin {
            return Err("admin".into());
        }
        Ok(())
    }

    fn can_deploy_to(
        &self,
        repo: &RepositoryConfig,
    ) -> Result<Option<MissingPermission>, InternalError> {
        let can_read = can_deploy(&self.permissions, repo)?;
        if can_read {
            Ok(None)
        } else {
            Ok(Some(MissingPermission("Write Repository".to_string())))
        }
    }

    fn can_read_from(
        &self,
        repo: &RepositoryConfig,
    ) -> Result<Option<MissingPermission>, InternalError> {
        match repo.visibility {
            Visibility::Public => Ok(None),
            Visibility::Private => {
                let can_read = can_read(&self.permissions, repo)?;
                if can_read {
                    Ok(None)
                } else {
                    Ok(Some(MissingPermission("Read Repository".to_string())))
                }
            }
            Visibility::Hidden => Ok(None),
        }
    }
}
