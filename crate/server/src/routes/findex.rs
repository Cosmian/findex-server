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
use tracing::{info, trace};

use crate::{
    core::{FindexServer, Role},
    error::server::FindexServerError,
    routes::error::{Response, ResponseBytes},
};

#[post("/indexes/{id}/fetch_entries")]
pub(crate) async fn fetch_entries(
    req: HttpRequest,
    id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> ResponseBytes {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /indexes/{id}/fetch_entries");

    if findex_server.get_access(&user, &id).await? < Role::Read {
        return Err(FindexServerError::Unauthorized(format!(
            "User {user} is not allowed to read index {id} (fetch_entries)",
        )));
    }

    let bytes = bytes.into_iter().collect::<Vec<_>>();

    let tokens = deserialize_token_set(&bytes)?;
    trace!("fetch_entries: number of tokens: {}:", tokens.len());

    // Collect into a vector to fix the order.
    let uids_and_values = findex_server
        .db
        .fetch_entries(&id.into_inner(), tokens)
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

#[post("/indexes/{id}/fetch_chains")]
pub(crate) async fn fetch_chains(
    req: HttpRequest,
    id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> ResponseBytes {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /indexes/{id}/fetch_chains");

    if findex_server.get_access(&user, &id).await? < Role::Read {
        return Err(FindexServerError::Unauthorized(format!(
            "User {user} is not allowed to read index {id} (fetch_chains)",
        )));
    }

    let bytes = bytes.into_iter().collect::<Vec<_>>();
    let tokens = deserialize_token_set(&bytes)?;
    trace!("fetch_chains: number of tokens: {}:", tokens.len());

    let uids_and_values = findex_server
        .db
        .fetch_chains(&id.into_inner(), tokens)
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

#[post("/indexes/{id}/upsert_entries")]
pub(crate) async fn upsert_entries(
    req: HttpRequest,
    id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> ResponseBytes {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /indexes/{id}/upsert_entries",);

    if findex_server.get_access(&user, &id).await? < Role::Write {
        return Err(FindexServerError::Unauthorized(format!(
            "User {user} is not allowed to write on index {id} (upsert_entries)",
        )));
    }

    let bytes = bytes.into_iter().collect::<Vec<_>>();
    let upsert_data = UpsertData::deserialize(&bytes)?;

    trace!("upsert_entries: num upsert data: {}", upsert_data.len());

    let rejected = findex_server
        .db
        .upsert_entries(&id.into_inner(), upsert_data)
        .await?;

    let bytes = rejected.serialize()?.to_vec();
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(bytes))
}

#[post("/indexes/{id}/insert_chains")]
pub(crate) async fn insert_chains(
    req: HttpRequest,
    id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> Response<()> {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /indexes/{id}/insert_chains",);

    if findex_server.get_access(&user, &id).await? < Role::Write {
        return Err(FindexServerError::Unauthorized(format!(
            "User {user} is not allowed to write on index {id} (insert_chains)",
        )));
    }

    let bytes = bytes.into_iter().collect::<Vec<_>>();
    let token_to_value_encrypted_value_map = TokenToEncryptedValueMap::deserialize(&bytes)?;

    findex_server
        .db
        .insert_chains(&id.into_inner(), token_to_value_encrypted_value_map)
        .await?;

    Ok(Json(()))
}

#[post("/indexes/{id}/delete_entries")]
pub(crate) async fn delete_entries(
    req: HttpRequest,
    id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> Response<()> {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /indexes/{id}/delete_entries",);

    if findex_server.get_access(&user, &id).await? < Role::Write {
        return Err(FindexServerError::Unauthorized(format!(
            "User {user} is not allowed to write on index {id} (delete_entries)",
        )));
    }

    let bytes = bytes.into_iter().collect::<Vec<_>>();
    let tokens = deserialize_token_set(&bytes)?;
    trace!("delete_entries: number of tokens: {}:", tokens.len());

    findex_server
        .db
        .delete(&id.into_inner(), FindexTable::Entry, tokens)
        .await?;

    Ok(Json(()))
}

#[post("/indexes/{id}/delete_chains")]
pub(crate) async fn delete_chains(
    req: HttpRequest,
    id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> Response<()> {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /indexes/{id}/delete_chains",);

    if findex_server.get_access(&user, &id).await? < Role::Write {
        return Err(FindexServerError::Unauthorized(format!(
            "User {user} is not allowed to write on index {id} (delete_chains)",
        )));
    }

    let bytes = bytes.into_iter().collect::<Vec<_>>();
    let tokens = deserialize_token_set(&bytes)?;
    trace!("delete_chains: number of tokens: {}:", tokens.len());

    findex_server
        .db
        .delete(&id.into_inner(), FindexTable::Chain, tokens)
        .await?;

    Ok(Json(()))
}

#[post("/indexes/{id}/dump_tokens")]
pub(crate) async fn dump_tokens(
    req: HttpRequest,
    id: web::Path<String>,
    findex_server: Data<Arc<FindexServer>>,
) -> ResponseBytes {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /indexes/{id}/dump_tokens");

    if findex_server.get_access(&user, &id).await? < Role::Read {
        return Err(FindexServerError::Unauthorized(format!(
            "User {user} is not allowed to read index {id} (dump_tokens)",
        )));
    }

    let tokens = findex_server.db.dump_tokens(&id.into_inner()).await?;
    trace!("dump_tokens: number of tokens: {}:", tokens.len());

    let bytes = tokens.serialize()?.to_vec();

    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(bytes))
}
