use std::sync::Arc;

use actix_web::{
    post,
    web::{self, Bytes, Data},
    HttpRequest, HttpResponse,
};
use cosmian_findex::{Address, MemoryADT, ADDRESS_LENGTH};
use cosmian_findex_structs::{
    Addresses, Guard, OptionalWords, Permission, Tasks, SERVER_ADDRESS_LENGTH, UID_LENGTH,
    WORD_LENGTH,
};
use tracing::trace;
use uuid::Uuid;

use crate::{
    core::FindexServer,
    error::server::FindexServerError,
    routes::{check_permission, error::ResponseBytes},
};

// TODO(hatem): reduce cloning

#[allow(clippy::indexing_slicing)]
fn prepend_index_id(
    address: &Address<ADDRESS_LENGTH>,
    index_id: &Uuid,
) -> Address<SERVER_ADDRESS_LENGTH> {
    let mut server_address = Address::<{ SERVER_ADDRESS_LENGTH }>::default();
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

    check_permission(&user, &index_id, Permission::Read, &findex_server).await?;

    // Parse index_id
    let index_id = Uuid::parse_str(&index_id)?;

    let bytes_slice = bytes.as_ref();
    let addresses = Addresses::deserialize(bytes_slice)?;

    let addresses = addresses
        .into_inner()
        .into_iter()
        .map(|a| prepend_index_id(&a, &index_id))
        .collect::<Vec<_>>();

    trace!("batch_read: number of addresses {}:", addresses.len());

    let result_words = OptionalWords::new(findex_server.db.batch_read(addresses).await?);
    trace!(
        "batch_read successful. Number of non null words: {}.",
        result_words
            .clone()
            .into_inner()
            .iter()
            .fold(0, |acc, x| acc + i32::from(x.is_some()))
    );

    // Convert Vec<Option<[u8; WORD_LENGTH]>> to Vec<u8>
    let response_bytes = Bytes::from(result_words.serialize()?);

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
    const INVALID_REQUEST: &str = "Invalid request.";
    let user = findex_server.get_user(&req);

    trace!("user {user}: POST /indexes/{index_id}/guarded_write");

    check_permission(&user, &index_id, Permission::Write, &findex_server).await?;

    // Parse index_id
    let index_id = Uuid::parse_str(&index_id)?;

    let error_prefix: String =
        format!("Invalid {OPERATION_NAME} request by {user} on index {index_id}.");

    let discriminant_flag = bytes.get(ADDRESS_LENGTH); // 0 or 1. 0 means None, 1 means Some. Assumes the first ADDRESS_LENGTH bytes are the address

    let flag = if let Some(f) = discriminant_flag {
        match *f {
            0 => false,
            1 => true,
            invalid => {
                trace!(
                    "{error_prefix} Invalid discriminant flag. Expected 0 or 1, found {invalid}"
                );
                return Err(FindexServerError::InvalidRequest(
                    INVALID_REQUEST.to_owned(),
                ));
            }
        }
    } else {
        trace!("{error_prefix} Invalid discriminant flag. Expected 0 or 1, found None");
        return Err(FindexServerError::InvalidRequest(
            INVALID_REQUEST.to_owned(),
        ));
    };

    let guard_len = if flag {
        ADDRESS_LENGTH + 1 + WORD_LENGTH
    } else {
        ADDRESS_LENGTH + 1
    };

    let guard = bytes.get(..guard_len);
    if guard.is_none() {
        trace!("{error_prefix} Could not parse guard.");
        return Err(FindexServerError::InvalidRequest(
            INVALID_REQUEST.to_owned(),
        ));
    }
    #[allow(clippy::unwrap_used)] // guard and tasks are checked to be Some just above
    let guard = Guard::deserialize(guard.unwrap())?;

    let tasks = bytes.get(guard_len..);
    if tasks.is_none() {
        trace!("{error_prefix} Could not parse tasks to be written.");
        return Err(FindexServerError::InvalidRequest(
            INVALID_REQUEST.to_owned(),
        ));
    }
    #[allow(clippy::unwrap_used)] // same as above, already checked to be Some
    let tasks = Tasks::deserialize(tasks.unwrap())?;

    let (a_g, w_g) = guard.into_inner();
    let bindings = tasks
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
