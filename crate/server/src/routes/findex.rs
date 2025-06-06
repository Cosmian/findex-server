use std::sync::Arc;

use actix_web::{
    HttpRequest, HttpResponse, post,
    web::{self, Bytes, Data},
};
use cosmian_findex_memories::reexport::cosmian_findex::{ADDRESS_LENGTH, Address, MemoryADT};

use cosmian_findex_structs::{
    Addresses, Bindings, CUSTOM_WORD_LENGTH, Guard, OptionalWords, Permission,
    SERVER_ADDRESS_LENGTH, UID_LENGTH,
};
use tracing::trace;
use uuid::Uuid;

use crate::{core::FindexServer, error::server::ServerError, routes::error::ResponseBytes};

#[allow(clippy::indexing_slicing)]
fn prepend_index_id(
    address: &Address<ADDRESS_LENGTH>,
    index_id: &Uuid,
) -> Address<SERVER_ADDRESS_LENGTH> {
    let mut server_address = Address::<{ SERVER_ADDRESS_LENGTH }>::from([0; SERVER_ADDRESS_LENGTH]);
    server_address[..UID_LENGTH].copy_from_slice(index_id.as_bytes());
    server_address[UID_LENGTH..].copy_from_slice(&**address);
    server_address
}

#[post("/indexes/{index_id}/batch_read")]
pub(crate) async fn findex_batch_read(
    req: HttpRequest,
    index_id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> ResponseBytes {
    let user = findex_server.get_user(&req);

    trace!("user {user}: POST /indexes/{index_id}/batch_read");

    findex_server
        .ensure_minimum_permission(&user, &index_id, Permission::Read)
        .await?;

    let index_id = Uuid::parse_str(&index_id)?;
    let addresses = Addresses::deserialize(&bytes)?
        .into_inner()
        .into_iter()
        .map(|a| prepend_index_id(&a, &index_id))
        .collect::<Vec<_>>();

    trace!("batch_read: number of addresses {}:", addresses.len());

    let words = findex_server.db.batch_read(addresses).await?;

    trace!(
        "batch_read successful. Number of non null words: {}.",
        words.iter().filter(|w| w.is_some()).count()
    );

    // Convert Vec<Option<[u8; WORD_LENGTH]>> to Vec<u8>
    let response_bytes = Bytes::from(OptionalWords::new(words).serialize()?);

    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(response_bytes))
}

#[post("/indexes/{index_id}/guarded_write")]
pub(crate) async fn findex_guarded_write(
    req: HttpRequest,
    index_id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> ResponseBytes {
    const OPERATION_NAME: &str = "guarded_write";
    let user = findex_server.get_user(&req);

    trace!("user {user}: POST /indexes/{index_id}/guarded_write");

    findex_server
        .ensure_minimum_permission(&user, &index_id, Permission::Write)
        .await?;

    let index_id = Uuid::parse_str(&index_id)?;

    let error_prefix = format!("Invalid {OPERATION_NAME} request by {user} on index {index_id}.");

    // 0 or 1. 0 means None, 1 means Some. Assumes the first ADDRESS_LENGTH
    // bytes are the address
    let guard_len = if let Some(f) = bytes.get(ADDRESS_LENGTH) {
        match *f {
            0 => ADDRESS_LENGTH + 1,
            1 => ADDRESS_LENGTH + 1 + CUSTOM_WORD_LENGTH,
            _ => {
                return Err(ServerError::InvalidRequest(format!(
                    "{error_prefix} Invalid discriminant flag. Expected 0 or 1, found {f}"
                )));
            }
        }
    } else {
        return Err(ServerError::InvalidRequest(format!(
            "{error_prefix} Invalid discriminant flag. Expected 0 or 1, found None"
        )));
    };

    let guard = Guard::deserialize(bytes.get(..guard_len).ok_or_else(|| {
        ServerError::InvalidRequest(format!("{error_prefix} Could not parse guard."))
    })?)?;

    let bindings = Bindings::deserialize(bytes.get(guard_len..).ok_or_else(|| {
        ServerError::InvalidRequest(format!(
            "{error_prefix} Could not parse bindings to be written.",
        ))
    })?)?;

    let (a_g, w_g) = guard.into_inner();
    let bindings = bindings
        .into_inner()
        .into_iter()
        .map(|(a, w)| (prepend_index_id(&a, &index_id), w))
        .collect::<Vec<_>>();

    let result_word = findex_server
        .db
        .guarded_write((prepend_index_id(&a_g, &index_id), w_g), bindings)
        .await?;

    let response_bytes = Bytes::from(OptionalWords::new(vec![result_word]).serialize()?);

    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(response_bytes))
}
