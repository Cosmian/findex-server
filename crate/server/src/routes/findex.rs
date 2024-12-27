use std::sync::Arc;

use actix_web::{
    post,
    web::{self, Bytes, Data, Json},
    HttpRequest, HttpResponse,
};
use cloudproof_findex::{
    db_interfaces::{redis::FindexTable, rest::UpsertData},
    reexport::{
        cosmian_crypto_core::bytes_ser_de::Serializable, cosmian_findex::TokenToEncryptedValueMap,
    },
    ser_de::ffi_ser_de::deserialize_token_set,
};
use cosmian_findex_structs::Permission;
use tracing::{info, trace};
use uuid::Uuid;

use crate::{
    core::FindexServer,
    routes::{
        check_permission,
        error::{Response, ResponseBytes},
    },
};

#[post("/indexes/{index_id}/fetch_entries")]
pub(crate) async fn findex_fetch_entries(
    req: HttpRequest,
    index_id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> ResponseBytes {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /indexes/{index_id}/fetch_entries");

    check_permission(&user, &index_id, Permission::Read, &findex_server).await?;

    let tokens = deserialize_token_set(&bytes)?;
    trace!("fetch_entries: number of tokens: {}:", tokens.len());

    // Parse index_id
    let index_id = Uuid::parse_str(&index_id)?;

    // Collect into a vector to fix the order.
    let uids_and_values = findex_server
        .db
        .findex_fetch_entries(&index_id, tokens)
        .await?;
    trace!(
        "fetch_entries: number of uids_and_values: {}:",
        uids_and_values.len()
    );

    let bytes = uids_and_values.serialize()?.to_vec();
    trace!("fetch_entries: number of bytes: {}:", bytes.len());

    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(bytes))
}

#[post("/indexes/{index_id}/fetch_chains")]
pub(crate) async fn findex_fetch_chains(
    req: HttpRequest,
    index_id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> ResponseBytes {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /indexes/{index_id}/fetch_chains");

    check_permission(&user, &index_id, Permission::Read, &findex_server).await?;

    let tokens = deserialize_token_set(&bytes)?;
    trace!("fetch_chains: number of tokens: {}:", tokens.len());

    // Parse index_id
    let index_id = Uuid::parse_str(&index_id)?;

    let uids_and_values = findex_server
        .db
        .findex_fetch_chains(&index_id, tokens)
        .await?;
    trace!(
        "fetch_chains: number of uids_and_values: {}:",
        uids_and_values.len()
    );

    let bytes = uids_and_values.serialize()?.to_vec();

    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(bytes))
}

#[post("/indexes/{index_id}/upsert_entries")]
pub(crate) async fn findex_upsert_entries(
    req: HttpRequest,
    index_id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> ResponseBytes {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /indexes/{index_id}/upsert_entries",);

    check_permission(&user, &index_id, Permission::Write, &findex_server).await?;

    let upsert_data = UpsertData::deserialize(&bytes)?;

    trace!("upsert_entries: num upsert data: {}", upsert_data.len());

    // Parse index_id
    let index_id = Uuid::parse_str(&index_id)?;

    let rejected = findex_server
        .db
        .findex_upsert_entries(&index_id, upsert_data)
        .await?;

    let bytes = rejected.serialize()?.to_vec();
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(bytes))
}

#[post("/indexes/{index_id}/insert_chains")]
pub(crate) async fn findex_insert_chains(
    req: HttpRequest,
    index_id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> Response<()> {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /indexes/{index_id}/insert_chains",);

    check_permission(&user, &index_id, Permission::Write, &findex_server).await?;

    // Parse index_id
    let index_id = Uuid::parse_str(&index_id)?;

    let token_to_value_encrypted_value_map = TokenToEncryptedValueMap::deserialize(&bytes)?;

    findex_server
        .db
        .findex_insert_chains(&index_id, token_to_value_encrypted_value_map)
        .await?;

    Ok(Json(()))
}

#[post("/indexes/{index_id}/delete_entries")]
pub(crate) async fn findex_delete_entries(
    req: HttpRequest,
    index_id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> Response<()> {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /indexes/{index_id}/delete_entries",);

    check_permission(&user, &index_id, Permission::Write, &findex_server).await?;

    let tokens = deserialize_token_set(&bytes)?;
    trace!("delete_entries: number of tokens: {}:", tokens.len());

    // Parse index_id
    let index_id = Uuid::parse_str(&index_id)?;

    findex_server
        .db
        .findex_delete(&index_id, FindexTable::Entry, tokens)
        .await?;

    Ok(Json(()))
}

#[post("/indexes/{index_id}/delete_chains")]
pub(crate) async fn findex_delete_chains(
    req: HttpRequest,
    index_id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> Response<()> {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /indexes/{index_id}/delete_chains",);

    check_permission(&user, &index_id, Permission::Write, &findex_server).await?;

    let tokens = deserialize_token_set(&bytes)?;
    trace!("delete_chains: number of tokens: {}:", tokens.len());

    // Parse index_id
    let index_id = Uuid::parse_str(&index_id)?;

    findex_server
        .db
        .findex_delete(&index_id, FindexTable::Chain, tokens)
        .await?;

    Ok(Json(()))
}

#[post("/indexes/{index_id}/dump_tokens")]
pub(crate) async fn findex_dump_tokens(
    req: HttpRequest,
    index_id: web::Path<String>,
    findex_server: Data<Arc<FindexServer>>,
) -> ResponseBytes {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /indexes/{index_id}/dump_tokens");

    check_permission(&user, &index_id, Permission::Read, &findex_server).await?;

    // Parse index_id
    let index_id = Uuid::parse_str(&index_id)?;

    let tokens = findex_server.db.findex_dump_tokens(&index_id).await?;
    trace!("dump_tokens: number of tokens: {}:", tokens.len());

    let bytes = tokens.serialize()?.to_vec();

    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(bytes))
}
// TODO(manu): put findex parameters in cli conf
