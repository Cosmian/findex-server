use std::{str::FromStr, sync::Arc};

use actix_web::{
    post,
    web::{self, Data, Json},
    HttpRequest,
};
use serde::{Deserialize, Serialize};
use tracing::{info, trace};

use crate::{
    core::{FindexServer, Role},
    error::{result::FResult, server::FindexServerError},
};

#[derive(Deserialize, Serialize, Debug)] // Debug is required by ok_json()
struct SuccessResponse {
    pub success: String,
}

#[post("/access/create")]
pub(crate) async fn create_access(
    req: HttpRequest,
    findex_server: Data<Arc<FindexServer>>,
) -> FResult<Json<SuccessResponse>> {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /access/create");

    // Check if the user has the right to grant access: only admins can do that
    let index_id = findex_server.db.create_access(&user).await?;
    trace!("New access successfully created: {index_id}");

    Ok(Json(SuccessResponse {
        success: format!("New access successfully created: {index_id}"),
    }))
}

#[post("/access/grant/{user_id}/{role}/{index_id}")]
pub(crate) async fn grant_access(
    req: HttpRequest,
    params: web::Path<(String, String, String)>,
    findex_server: Data<Arc<FindexServer>>,
) -> FResult<Json<SuccessResponse>> {
    let user = findex_server.get_user(&req);
    let (user_id, role, index_id) = params.into_inner();
    info!("user {user}: POST /access/grant/{user_id}/{role}/{index_id}");

    // Check if the user has the right to grant access: only admins can do that
    let user_role = findex_server.get_access(&user, &index_id).await?;
    if Role::Admin != user_role {
        return Err(FindexServerError::Unauthorized(format!(
            "Delegating access to an index requires an admin role. User {user} with role \
             {user_role} does not allow granting access to index {index_id} with role {role}",
        )));
    }

    findex_server
        .db
        .grant_access(&user_id, Role::from_str(role.as_str())?, &index_id)
        .await?;

    Ok(Json(SuccessResponse {
        success: format!("Access for {user_id} on index {index_id} successfully added"),
    }))
}

#[post("/access/revoke/{user_id}/{index_id}")]
pub(crate) async fn revoke_access(
    req: HttpRequest,
    params: web::Path<(String, String)>,
    findex_server: Data<Arc<FindexServer>>,
) -> FResult<Json<SuccessResponse>> {
    let user = findex_server.get_user(&req);
    let (user_id, index_id) = params.into_inner();
    info!("user {user}: POST /access/revoke/{user_id}/{index_id}");

    // Check if the user has the right to revoke access: only admins can do that
    let user_role = findex_server.get_access(&user, &index_id).await?;
    if Role::Admin != user_role {
        return Err(FindexServerError::Unauthorized(format!(
            "Revoking access to an index requires an admin role. User {user} with role \
             {user_role} does not allow revoking access to index {index_id}",
        )));
    }

    findex_server.db.revoke_access(&user_id).await?;

    Ok(Json(SuccessResponse {
        success: format!("Access for {user_id} on index {index_id} successfully added"),
    }))
}
