use actix_web::{get, patch, post, web, HttpRequest};
use sea_orm::ActiveValue::Set;
use sea_orm::{DatabaseConnection, QueryFilter};
use serde::{Deserialize, Serialize};

use crate::api_response::{APIResponse, NRResponse};

use crate::authentication::Authentication;
use crate::error::internal_error::InternalError;
use crate::system::models::UserListResponse;
use crate::system::permissions::options::CanIDo;
use crate::system::permissions::UserPermissions;
use crate::system::user;
use crate::system::user::{UserEntity, UserModel};
use crate::system::utils::{hash, NewPassword, NewUser};
use crate::utils::get_current_time;
use sea_orm::ActiveModelTrait;
use sea_orm::ColumnTrait;
use sea_orm::EntityTrait;
use sea_orm::IntoActiveModel;

#[get("/api/admin/user/list")]
pub async fn list_users(
    database: web::Data<DatabaseConnection>,
    auth: Authentication,
    r: HttpRequest,
) -> NRResponse {
    let caller: UserModel = auth.get_user(&database).await??;
    caller.can_i_edit_users()?;
    let vec = UserEntity::find()
        .into_model::<UserListResponse>()
        .all(database.as_ref())
        .await
        .map_err(InternalError::from)?;

    APIResponse::from(Some(vec))
}

#[get("/api/admin/user/get/{user}")]
pub async fn get_user(
    database: web::Data<DatabaseConnection>,
    r: HttpRequest,
    auth: Authentication,
    path: web::Path<i64>,
) -> NRResponse {
    let caller: UserModel = auth.get_user(&database).await??;
    caller.can_i_edit_users()?;
    let user = UserEntity::find_by_id(path.into_inner())
        .one(database.as_ref())
        .await
        .map_err(InternalError::from)?;

    APIResponse::from(Some(user))
}

#[post("/api/admin/user/add")]
pub async fn add_user(
    database: web::Data<DatabaseConnection>,
    r: HttpRequest,
    auth: Authentication,
    nc: web::Json<NewUser>,
) -> NRResponse {
    let caller: UserModel = auth.get_user(&database).await??;
    caller.can_i_edit_users()?;
    if user::get_by_username(&nc.0.username, &database)
        .await?
        .is_some()
    {
        return already_exists_what("username");
    }
    if UserEntity::find()
        .filter(user::database::Column::Email.eq(nc.email.clone()))
        .one(database.as_ref())
        .await
        .map_err(InternalError::from)?
        .is_some()
    {
        return already_exists_what("email");
    }
    let user = user::database::ActiveModel {
        id: Default::default(),
        name: Set(nc.0.name),
        username: Set(nc.0.username.clone()),
        email: Set(nc.0.email),
        password: Set(hash(nc.0.password)?),
        permissions: Set(UserPermissions::default()),
        created: Set(get_current_time()),
    };
    UserEntity::insert(user)
        .exec(database.as_ref())
        .await
        .map_err(InternalError::from)?;
    Ok(APIResponse::ok())
}

#[patch("/api/admin/user/{user}/modify")]
pub async fn modify_user(
    database: web::Data<DatabaseConnection>,
    r: HttpRequest,
    auth: Authentication,
    path: web::Path<i64>,
    nc: web::Json<user::database::ModifyUser>,
) -> NRResponse {
    let caller: UserModel = auth.get_user(&database).await??;
    caller.can_i_edit_users()?;
    let user = UserEntity::find_by_id(path.into_inner())
        .one(database.as_ref())
        .await
        .map_err(InternalError::from)?;
    if user.is_none() {
        return not_found();
    }
    let model = nc.0.into_active_model();
    let user = model
        .update(database.as_ref())
        .await
        .map_err(InternalError::from)?;
    //update_user(user.unwrap().id, &nc.email, &nc.name, &database)?;
    Ok(APIResponse::ok())
}

#[patch("/api/admin/user/{user}/modify/permissions")]
pub async fn update_permission(
    database: web::Data<DatabaseConnection>,
    r: HttpRequest,
    auth: Authentication,
    permissions: web::Json<UserPermissions>,
    path: web::Path<i64>,
) -> NRResponse {
    let caller: UserModel = auth.get_user(&database).await??;
    caller.can_i_edit_users()?;

    let user = UserEntity::find_by_id(path.into_inner())
        .one(database.as_ref())
        .await
        .map_err(InternalError::from)?;
    if user.is_none() {
        return not_found();
    }
    let user: UserModel = user.unwrap();
    let mut user_active: user::database::ActiveModel = user.into_active_model();

    user_active.permissions = Set(permissions.0);

    let user = user_active
        .update(database.as_ref())
        .await
        .map_err(InternalError::from)?;
    Ok(APIResponse::ok())
}

#[post("/api/admin/user/{user}/password")]
pub async fn change_password(
    database: web::Data<DatabaseConnection>,
    r: HttpRequest,
    auth: Authentication,
    path: web::Path<i64>,
    nc: web::Json<NewPassword>,
) -> NRResponse {
    let caller: UserModel = auth.get_user(&database).await??;
    caller.can_i_edit_users()?;
    let user = UserEntity::find_by_id(path.into_inner())
        .one(database.as_ref())
        .await
        .map_err(InternalError::from)?;
    if user.is_none() {
        return not_found();
    }
    let user: UserModel = user.unwrap();
    let hashed_password: String = hash(nc.0.password)?;
    let mut user_active: user::database::ActiveModel = user.into_active_model();

    user_active.password = Set(hashed_password);

    let user = user_active
        .update(database.as_ref())
        .await
        .map_err(InternalError::from)?;
    Ok(APIResponse::ok())
}

#[get("/api/admin/user/{user}/delete")]
pub async fn delete_user(
    database: web::Data<DatabaseConnection>,
    r: HttpRequest,
    user: web::Path<i64>,
    auth: Authentication,
) -> NRResponse {
    let caller: UserModel = auth.get_user(&database).await??;
    caller.can_i_edit_users()?;
    let user = user.into_inner();

    UserEntity::delete_by_id(user)
        .exec(database.as_ref())
        .await
        .map_err(InternalError::from)?;
    Ok(APIResponse::ok())
}
