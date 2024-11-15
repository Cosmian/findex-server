use std::{str::FromStr, sync::Arc};

use actix_web::{
    post,
    web::{self, Data, Json},
    HttpRequest,
};
use cosmian_findex_structs::Permission;
use tracing::{debug, info};

use crate::{
    core::FindexServer,
    error::{result::FResult, server::FindexServerError},
    routes::{error::SuccessResponse, get_index_id},
};

pub(crate) async fn check_permission(
    user: &str,
    index_id: &str,
    expected_permission: Permission,
    findex_server: &FindexServer,
) -> FResult<()> {
    let permission = findex_server.get_permission(user, index_id).await?;
    debug!("check_permission: user {user} has permission {permission} on index {index_id}");
    if permission < expected_permission {
        return Err(FindexServerError::Unauthorized(format!(
            "User {user} with permission {permission} is not allowed to write on index {index_id}",
        )));
    }
    Ok(())
}

#[post("/create/index")]
pub(crate) async fn create_index_id(
    req: HttpRequest,
    findex_server: Data<Arc<FindexServer>>,
) -> FResult<Json<SuccessResponse>> {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /permission/create");

    // Check if the user has the right to grant permission: only admins can do that
    let index_id = findex_server.db.create_index_id(&user).await?;

    Ok(Json(SuccessResponse {
        success: format!("[{user}] New admin permission successfully created on index: {index_id}"),
    }))
}

#[post("/permission/grant/{user_id}/{permission}/{index_id}")]
pub(crate) async fn grant_permission(
    req: HttpRequest,
    params: web::Path<(String, String, String)>,
    findex_server: Data<Arc<FindexServer>>,
) -> FResult<Json<SuccessResponse>> {
    let user = findex_server.get_user(&req);
    let (user_id, permission, index_id) = params.into_inner();
    info!("user {user}: POST /permission/grant/{user_id}/{permission}/{index_id}");

    // Check if the user has the right to grant permission: only admins can do that
    let user_permission = findex_server.get_permission(&user, &index_id).await?;
    if Permission::Admin != user_permission {
        return Err(FindexServerError::Unauthorized(format!(
            "Delegating permission to an index requires an admin permission. User {user} with \
             permission {user_permission} does not allow granting permission to index {index_id} \
             with permission {permission}",
        )));
    }

    findex_server
        .db
        .grant_permission(
            &user_id,
            Permission::from_str(permission.as_str())?,
            &get_index_id(&index_id)?,
        )
        .await?;

    Ok(Json(SuccessResponse {
        success: format!(
            "[{user_id}] permission {permission} on index {index_id} successfully added"
        ),
    }))
}

#[post("/permission/revoke/{user_id}/{index_id}")]
pub(crate) async fn revoke_permission(
    req: HttpRequest,
    params: web::Path<(String, String)>,
    findex_server: Data<Arc<FindexServer>>,
) -> FResult<Json<SuccessResponse>> {
    let user = findex_server.get_user(&req);
    let (user_id, index_id) = params.into_inner();
    info!("user {user}: POST /permission/revoke/{user_id}/{index_id}");

    // Check if the user has the right to revoke permission: only admins can do that
    let user_permission = findex_server.get_permission(&user, &index_id).await?;
    if Permission::Admin != user_permission {
        return Err(FindexServerError::Unauthorized(format!(
            "Revoking permission to an index requires an admin permission. User {user} with \
             permission {user_permission} does not allow revoking permission to index {index_id}",
        )));
    }

    findex_server
        .db
        .revoke_permission(&user_id, &get_index_id(&index_id)?)
        .await?;

    Ok(Json(SuccessResponse {
        success: format!("Permission for {user_id} on index {index_id} successfully added"),
    }))
}
