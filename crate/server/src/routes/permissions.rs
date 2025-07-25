use std::{str::FromStr, sync::Arc};

use actix_web::{
    HttpRequest, HttpResponse, post,
    web::{self, Data, Json},
};
use cosmian_crypto_core::bytes_ser_de::Serializable;
use cosmian_findex_structs::Permission;
use tracing::trace;
use uuid::Uuid;

use crate::{
    core::FindexServer,
    database::database_traits::PermissionsTrait,
    error::{result::FResult, server::ServerError},
    routes::error::{ResponseBytes, SuccessResponse},
};

#[post("/create/index")]
pub(crate) async fn create_index_id(
    req: HttpRequest,
    findex_server: Data<Arc<FindexServer>>,
) -> FResult<Json<SuccessResponse>> {
    let user = findex_server.get_user(&req);
    trace!("user {user}: POST /permission/create");

    let index_id = findex_server.db.create_index_id(&user).await?;

    Ok(Json(SuccessResponse {
        success: format!("[{user}] New admin permission successfully created on index: {index_id}"),
        index_id,
    }))
}

#[post("/permission/set/{user_id}/{permission}/{index_id}")]
pub(crate) async fn set_permission(
    req: HttpRequest,
    params: web::Path<(String, String, String)>,
    findex_server: Data<Arc<FindexServer>>,
) -> FResult<Json<SuccessResponse>> {
    let user = findex_server.get_user(&req);
    let (user_id, permission, index_id) = params.into_inner();
    trace!("user {user}: POST /permission/set/{user_id}/{permission}/{index_id}");

    // Check if the user has the right to set permission: only admins can do that
    let user_permission = findex_server.get_permission(&user, &index_id).await?;
    if Permission::Admin != user_permission {
        return Err(ServerError::Unauthorized(format!(
            "Delegating permission to an index requires an admin permission. User {user} with \
             permission {user_permission} does not allow setting permission to index {index_id} \
             with permission {permission}",
        )));
    }

    // Parse index_id
    let index_id = Uuid::parse_str(&index_id)?;

    findex_server
        .db
        .set_permission(
            &user_id,
            Permission::from_str(permission.as_str())?,
            &index_id,
        )
        .await?;

    Ok(Json(SuccessResponse {
        success: format!(
            "[{user_id}] permission {permission} on index {index_id} successfully added"
        ),
        index_id,
    }))
}

#[post("/permission/list/{user_id}")]
pub(crate) async fn list_permission(
    req: HttpRequest,
    params: web::Path<String>,
    findex_server: Data<Arc<FindexServer>>,
) -> ResponseBytes {
    let request_user = findex_server.get_user(&req);
    let requested_user_id = params.into_inner();

    trace!("user {request_user}: POST /permission/list/{requested_user_id}");

    let request_user_permissions = findex_server.db.get_permissions(&request_user).await?;
    let requested_user_permissions = findex_server.db.get_permissions(&requested_user_id).await?;

    // To avoid a user to lookup who are the more powerful users only display the
    // minimum permission between the two users
    let min_permissions = requested_user_permissions.min(&request_user_permissions);

    let bytes = min_permissions.serialize()?;
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(bytes.to_vec()))
}

#[post("/permission/revoke/{user_id}/{index_id}")]
pub(crate) async fn revoke_permission(
    req: HttpRequest,
    params: web::Path<(String, String)>,
    findex_server: Data<Arc<FindexServer>>,
) -> FResult<Json<SuccessResponse>> {
    let user = findex_server.get_user(&req);
    let (user_id, index_id) = params.into_inner();

    trace!("user {user}: POST /permission/revoke/{user_id}/{index_id}");

    // Check if the user has the right to revoke permission: only admins can do that
    let user_permission = findex_server.get_permission(&user, &index_id).await?;

    if Permission::Admin != user_permission {
        return Err(ServerError::Unauthorized(format!(
            "Revoking permission to an index requires an admin permission. User {user} with \
             permission {user_permission} does not allow revoking permission to index {index_id}",
        )));
    }

    let index_id = Uuid::parse_str(&index_id)?;

    findex_server
        .db
        .revoke_permission(&user_id, &index_id)
        .await?;

    Ok(Json(SuccessResponse {
        success: format!("Permission for {user_id} on index {index_id} successfully added"),
        index_id,
    }))
}
