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
use tracing::{debug, info, trace};

use crate::{
    core::{FindexServer, Permission},
    error::server::FindexServerError,
    routes::{
        error::{Response, ResponseBytes},
        get_index_id,
    },
};

#[post("/indexes/{index_id}/fetch_entries")]
pub(crate) async fn fetch_entries(
    req: HttpRequest,
    index_id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> ResponseBytes {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /indexes/{index_id}/fetch_entries");

    // todo(manu): log permission
    if findex_server.get_permission(&user, &index_id).await? < Permission::Read {
        return Err(FindexServerError::Unauthorized(format!(
            "User {user} is not allowed to read index {index_id} (fetch_entries)",
        )));
    }

    let bytes = bytes.into_iter().collect::<Vec<_>>();

    let tokens = deserialize_token_set(&bytes)?;
    trace!("fetch_entries: number of tokens: {}:", tokens.len());

    // Collect into a vector to fix the order.
    let uids_and_values = findex_server
        .db
        .fetch_entries(&get_index_id(index_id.as_str())?, tokens)
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
pub(crate) async fn fetch_chains(
    req: HttpRequest,
    index_id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> ResponseBytes {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /indexes/{index_id}/fetch_chains");

    if findex_server.get_permission(&user, &index_id).await? < Permission::Read {
        return Err(FindexServerError::Unauthorized(format!(
            "User {user} is not allowed to read index {index_id} (fetch_chains)",
        )));
    }

    let bytes = bytes.into_iter().collect::<Vec<_>>();
    let tokens = deserialize_token_set(&bytes)?;
    trace!("fetch_chains: number of tokens: {}:", tokens.len());

    let uids_and_values = findex_server
        .db
        .fetch_chains(&get_index_id(index_id.as_str())?, tokens)
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
pub(crate) async fn upsert_entries(
    req: HttpRequest,
    index_id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> ResponseBytes {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /indexes/{index_id}/upsert_entries",);

    let user_permission = findex_server.get_permission(&user, &index_id).await?;
    debug!("user {user} has permission: {user_permission}");
    if user_permission < Permission::Write {
        return Err(FindexServerError::Unauthorized(format!(
            "User {user} is not allowed to write on index {index_id} (upsert_entries)",
        )));
    }

    let bytes = bytes.into_iter().collect::<Vec<_>>();
    let upsert_data = UpsertData::deserialize(&bytes)?;

    trace!("upsert_entries: num upsert data: {}", upsert_data.len());

    let rejected = findex_server
        .db
        .upsert_entries(&get_index_id(index_id.as_str())?, upsert_data)
        .await?;

    let bytes = rejected.serialize()?.to_vec();
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(bytes))
}

#[post("/indexes/{index_id}/insert_chains")]
pub(crate) async fn insert_chains(
    req: HttpRequest,
    index_id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> Response<()> {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /indexes/{index_id}/insert_chains",);

    if findex_server.get_permission(&user, &index_id).await? < Permission::Write {
        return Err(FindexServerError::Unauthorized(format!(
            "User {user} is not allowed to write on index {index_id} (insert_chains)",
        )));
    }

    let bytes = bytes.into_iter().collect::<Vec<_>>();
    let token_to_value_encrypted_value_map = TokenToEncryptedValueMap::deserialize(&bytes)?;

    findex_server
        .db
        .insert_chains(
            &get_index_id(index_id.as_str())?,
            token_to_value_encrypted_value_map,
        )
        .await?;

    Ok(Json(()))
}

#[post("/indexes/{index_id}/delete_entries")]
pub(crate) async fn delete_entries(
    req: HttpRequest,
    index_id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> Response<()> {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /indexes/{index_id}/delete_entries",);

    if findex_server.get_permission(&user, &index_id).await? < Permission::Write {
        return Err(FindexServerError::Unauthorized(format!(
            "User {user} is not allowed to write on index {index_id} (delete_entries)",
        )));
    }

    let bytes = bytes.into_iter().collect::<Vec<_>>();
    let tokens = deserialize_token_set(&bytes)?;
    trace!("delete_entries: number of tokens: {}:", tokens.len());

    findex_server
        .db
        .delete(
            &get_index_id(index_id.as_str())?,
            FindexTable::Entry,
            tokens,
        )
        .await?;

    Ok(Json(()))
}

#[post("/indexes/{index_id}/delete_chains")]
pub(crate) async fn delete_chains(
    req: HttpRequest,
    index_id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> Response<()> {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /indexes/{index_id}/delete_chains",);

    if findex_server.get_permission(&user, &index_id).await? < Permission::Write {
        return Err(FindexServerError::Unauthorized(format!(
            "User {user} is not allowed to write on index {index_id} (delete_chains)",
        )));
    }

    let bytes = bytes.into_iter().collect::<Vec<_>>();
    let tokens = deserialize_token_set(&bytes)?;
    trace!("delete_chains: number of tokens: {}:", tokens.len());

    findex_server
        .db
        .delete(
            &get_index_id(index_id.as_str())?,
            FindexTable::Chain,
            tokens,
        )
        .await?;

    Ok(Json(()))
}

#[post("/indexes/{index_id}/dump_tokens")]
pub(crate) async fn dump_tokens(
    req: HttpRequest,
    index_id: web::Path<String>,
    findex_server: Data<Arc<FindexServer>>,
) -> ResponseBytes {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /indexes/{index_id}/dump_tokens");

    if findex_server.get_permission(&user, &index_id).await? < Permission::Read {
        return Err(FindexServerError::Unauthorized(format!(
            "User {user} is not allowed to read index {index_id} (dump_tokens)",
        )));
    }

    let tokens = findex_server
        .db
        .dump_tokens(&get_index_id(index_id.as_str())?)
        .await?;
    trace!("dump_tokens: number of tokens: {}:", tokens.len());

    let bytes = tokens.serialize()?.to_vec();

    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(bytes))
}
// todo(manu): put findex parameters in cli conf
