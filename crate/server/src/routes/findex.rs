use crate::{
    core::FindexServer,
    routes::error::{Response, ResponseBytes},
};
use actix_web::{
    post,
    web::{Bytes, Data, Json},
    HttpRequest, HttpResponse,
};
use cloudproof_findex::{
    db_interfaces::{redis::FindexTable, rest::UpsertData},
    reexport::{
        cosmian_crypto_core::bytes_ser_de::Serializable, cosmian_findex::TokenToEncryptedValueMap,
    },
    ser_de::ffi_ser_de::deserialize_token_set,
};
use std::sync::Arc;
use tracing::{info, trace};

#[post("/indexes/fetch_entries")]
pub(crate) async fn fetch_entries(
    req: HttpRequest,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> ResponseBytes {
    info!("POST /fetch_entries {}", findex_server.get_user(&req));
    let bytes = bytes.into_iter().collect::<Vec<_>>();
    let tokens = deserialize_token_set(&bytes)?;
    trace!("fetch_entries: number of tokens: {}:", tokens.len());

    // Collect into a vector to fix the order.
    let uids_and_values = findex_server.db.fetch_entries(tokens).await?;
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

#[post("/indexes/fetch_chains")]
pub(crate) async fn fetch_chains(
    req: HttpRequest,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> ResponseBytes {
    info!("POST /fetch_chains {}", findex_server.get_user(&req));

    let bytes = bytes.into_iter().collect::<Vec<_>>();
    let tokens = deserialize_token_set(&bytes)?;
    trace!("fetch_chains: number of tokens: {}:", tokens.len());

    let uids_and_values = findex_server.db.fetch_chains(tokens).await?;
    trace!(
        "fetch_chains: number of uids_and_values: {}:",
        uids_and_values.len()
    );

    let bytes = uids_and_values.serialize()?.to_vec();

    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(bytes))
}

#[post("/indexes/upsert_entries")]
pub(crate) async fn upsert_entries(
    req: HttpRequest,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> ResponseBytes {
    info!("POST /upsert_entries {}", findex_server.get_user(&req));
    let bytes = bytes.into_iter().collect::<Vec<_>>();
    let upsert_data = UpsertData::deserialize(&bytes)?;

    trace!("upsert_entries: num upsert data: {}", upsert_data.len());

    let rejected = findex_server.db.upsert_entries(upsert_data).await?;

    let bytes = rejected.serialize()?.to_vec();
    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(bytes))
}

#[post("/indexes/insert_chains")]
pub(crate) async fn insert_chains(
    req: HttpRequest,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> Response<()> {
    info!("POST /insert_chains {}", findex_server.get_user(&req));
    let bytes = bytes.into_iter().collect::<Vec<_>>();
    let token_to_value_encrypted_value_map = TokenToEncryptedValueMap::deserialize(&bytes)?;

    findex_server
        .db
        .insert_chains(token_to_value_encrypted_value_map)
        .await?;

    Ok(Json(()))
}

#[post("/indexes/delete_entries")]
pub(crate) async fn delete_entries(
    req: HttpRequest,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> Response<()> {
    info!("POST /delete_entries {}", findex_server.get_user(&req));
    let bytes = bytes.into_iter().collect::<Vec<_>>();
    let tokens = deserialize_token_set(&bytes)?;
    trace!("delete_entries: number of tokens: {}:", tokens.len());

    findex_server.db.delete(FindexTable::Entry, tokens).await?;

    Ok(Json(()))
}

#[post("/indexes/delete_chains")]
pub(crate) async fn delete_chains(
    req: HttpRequest,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> Response<()> {
    info!("POST /delete_chains {}", findex_server.get_user(&req));
    let bytes = bytes.into_iter().collect::<Vec<_>>();
    let tokens = deserialize_token_set(&bytes)?;
    trace!("delete_chains: number of tokens: {}:", tokens.len());

    findex_server.db.delete(FindexTable::Chain, tokens).await?;

    Ok(Json(()))
}

#[post("/indexes/dump_tokens")]
pub(crate) async fn dump_tokens(
    req: HttpRequest,
    findex_server: Data<Arc<FindexServer>>,
) -> ResponseBytes {
    info!("POST /dump_tokens {}", findex_server.get_user(&req));
    let tokens = findex_server.db.dump_tokens().await?;
    trace!("dump_tokens: number of tokens: {}:", tokens.len());

    let bytes = tokens.serialize()?.to_vec();

    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(bytes))
}
