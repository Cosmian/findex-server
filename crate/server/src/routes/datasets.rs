use std::sync::Arc;

use actix_web::{
    post,
    web::{self, Bytes, Data, Json},
    HttpRequest, HttpResponse,
};
use cosmian_crypto_core::bytes_ser_de::Serializable;
use cosmian_findex_structs::{EncryptedEntries, Permission, Uuids};
use tracing::{info, trace};
use uuid::Uuid;

use crate::{
    core::FindexServer,
    database::database_traits::DatasetsTrait,
    error::result::FResult,
    routes::error::{ResponseBytes, SuccessResponse},
};

#[post("/datasets/{index_id}/add_entries")]
pub(crate) async fn datasets_add_entries(
    req: HttpRequest,
    index_id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> FResult<Json<SuccessResponse>> {
    let user = findex_server.get_user(&req);

    info!("user {user}: POST /datasets/{index_id}/add_entries");

    findex_server
        .check_permission(&user, &index_id, Permission::Write)
        .await?;

    let index_id = Uuid::parse_str(&index_id)?;
    let encrypted_entries = EncryptedEntries::deserialize(&bytes)?;

    trace!(
        "add_entries: number of encrypted entries: {}:",
        encrypted_entries.len()
    );

    findex_server
        .db
        .dataset_add_entries(&index_id, &encrypted_entries)
        .await?;

    Ok(Json(SuccessResponse {
        success: format!(
            "{} entries successfully added to index {index_id}",
            encrypted_entries.len()
        ),
        index_id,
    }))
}

#[post("/datasets/{index_id}/delete_entries")]
pub(crate) async fn datasets_del_entries(
    req: HttpRequest,
    index_id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> FResult<Json<SuccessResponse>> {
    let user = findex_server.get_user(&req);

    info!("user {user}: POST /datasets/{index_id}/delete_entries");

    findex_server
        .check_permission(&user, &index_id, Permission::Write)
        .await?;

    let index_id = Uuid::parse_str(&index_id)?;
    let uuids = Uuids::deserialize(&bytes)?;

    trace!(
        "delete_entries: number of UUIDs: {} from index {}:",
        uuids.len(),
        index_id
    );

    findex_server
        .db
        .dataset_delete_entries(&index_id, &uuids)
        .await?;

    Ok(Json(SuccessResponse {
        success: format!(
            "Encrypted entries successfully deleted from index {index_id}. UUIDs were {uuids}",
        ),
        index_id,
    }))
}

#[post("/datasets/{index_id}/get_entries")]
pub(crate) async fn datasets_get_entries(
    req: HttpRequest,
    index_id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> ResponseBytes {
    let user = findex_server.get_user(&req);

    info!("user {user}: POST /datasets/{index_id}/get_entries",);

    findex_server
        .check_permission(&user, &index_id, Permission::Read)
        .await?;

    let index_id = Uuid::parse_str(&index_id)?;
    let uuids = Uuids::deserialize(&bytes)?;

    trace!("get_entries: number of UUIDs: {}:", uuids.len());

    let encrypted_entries = findex_server
        .db
        .dataset_get_entries(&index_id, &uuids)
        .await?;

    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(encrypted_entries.serialize()?.to_vec()))
}
