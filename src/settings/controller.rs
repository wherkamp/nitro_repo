use actix_web::{get, post, web, HttpRequest, Responder};
use serde::{Deserialize, Serialize};

use crate::api_response::{APIResponse, SiteResponse};

use crate::error::internal_error::InternalError;
use crate::error::request_error::RequestError;
use crate::settings::action::get_setting;
use crate::settings::settings::{DBSetting, SettingManager};
use crate::settings::utils::get_setting_report;
use crate::system::utils::get_user_by_header;
use crate::utils::{get_current_time};
use crate::{settings, DbPool};
use diesel::MysqlConnection;
use crate::error::response::unauthorized;

pub fn get_setting_or_empty(
    string: &str,
    connection: &MysqlConnection,
) -> Result<DBSetting, InternalError> {
    let result = get_setting(string.clone(), &connection)?;
    if let Some(some) = result {
        return Ok(some);
    } else {
        return default_setting(string);
    }
}

pub fn default_setting(string: &str) -> Result<DBSetting, InternalError> {
    let setting = SettingManager::get_setting(string.to_string())
        .ok_or(InternalError::Error(
            "Unable to find setting".to_string(),
        ))
        .unwrap();
    return Ok(DBSetting {
        id: 0,
        setting: setting.clone(),
        value: setting.default.unwrap_or_else(default_string),
        updated: get_current_time(),
    });
}

pub fn default_string() -> String {
    return "".to_string();
}

#[get("/api/setting/{setting}")]
pub async fn about_setting(
    pool: web::Data<DbPool>,
    r: HttpRequest,
    web::Path(setting): web::Path<String>,
) -> SiteResponse {
    let connection = pool.get()?;


    let option = get_setting_or_empty(setting.as_str(), &connection)?;
    if !option.setting.public.unwrap_or(false) {
        //TODO check if admin
        return unauthorized();
    }
    return APIResponse::from(Some(option)).respond(&r);
}

#[get("/api/settings/report")]
pub async fn setting_report(
    pool: web::Data<DbPool>,
    r: HttpRequest,
) -> SiteResponse {
    let connection = pool.get()?;

    let user =
        get_user_by_header(r.headers(), &connection)?;
    if user.is_none() || !user.unwrap().permissions.admin {
        return unauthorized();
    }
    let report = get_setting_report(&connection)?;
    return APIResponse::from(Some(report)).respond(&r);
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateSettingRequest {
    pub value: String,
}

#[post("/api/admin/setting/{setting}/update")]
pub async fn update_setting(
    pool: web::Data<DbPool>,
    r: HttpRequest,
    request: web::Json<UpdateSettingRequest>,
    web::Path(setting): web::Path<String>,
) -> SiteResponse {
    let connection = pool.get()?;

    let user =
        get_user_by_header(r.headers(), &connection)?;
    if user.is_none() || !user.unwrap().permissions.admin {
        return unauthorized();
    }
    let mut option = get_setting_or_empty(setting.as_str(), &connection)?;
    option.set_value(request.value.clone());
    settings::action::update_setting(&option, &connection)?;
    let option =
        get_setting(setting.as_str(), &connection)?;
    return APIResponse::respond_new(option, &r);
}
