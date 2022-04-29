use actix_web::cookie::Cookie;
use actix_web::{post, web, HttpRequest};
use log::error;

use crate::api_response::{APIResponse, SiteResponse};

use crate::error::response::unauthorized;

use crate::system::utils::verify_login;
use crate::utils::get_current_time;

use sea_orm::ActiveValue::Set;
use sea_orm::EntityTrait;
use sea_orm::{DatabaseConnection, NotSet};
use serde::{Deserialize, Serialize};
use crate::authentication::session::SessionManager;
use crate::authentication::session::SessionManagerType;
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Login {
    pub username: String,
    pub password: String,
}

#[post("/api/login")]
pub async fn login(
    connection: web::Data<DatabaseConnection>,
    session_manager: web::Data<SessionManager>,
    r: HttpRequest,
    nc: web::Json<Login>,
) -> SiteResponse {
    let _username = nc.username.clone();
    if let Some(user) = verify_login(nc.username.clone(), nc.password.clone(), &connection).await? {
        let cookie: Cookie = r.cookie("session").unwrap();
        actix_web::rt::spawn(async move {
            if let Err(_) = session_manager.set_user(cookie.value(), user.id).await {
                error!("Unable to save user {} to cookie {}", user.id, cookie.value());
            }
        });
        APIResponse::respond_new(Some(true), &r)
    } else {
        unauthorized()
    }
}
