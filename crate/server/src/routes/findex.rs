use std::sync::Arc;

use actix_web::{
    post,
    web::{self, Bytes, Data},
    HttpRequest, HttpResponse,
};
use cosmian_findex::{MemoryADT, ADDRESS_LENGTH, WORD_LENGTH};
use cosmian_findex_structs::{Addresses, Guard, OptionalWords, Permission, Tasks};
use tracing::{info, trace};

use crate::{
    core::FindexServer,
    error::server::FindexServerError,
    routes::{check_permission, error::ResponseBytes},
};

// todo(hatem): reduce cloning

#[post("/indexes/{index_id}/batch_read")]
pub(crate) async fn findex_batch_read(
    req: HttpRequest,
    index_id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> ResponseBytes {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /indexes/{index_id}/batch_read");

    check_permission(&user, &index_id, Permission::Read, &findex_server).await?;

    let bytes_slice = bytes.as_ref();
    let addresses = Addresses::deserialize(bytes_slice)?;

    trace!(
        "batch_read: number of addresses {}:",
        addresses.clone().into_inner().len()
    );

    let result_words =
        OptionalWords::new(findex_server.db.batch_read(addresses.into_inner()).await?);
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
    let user = findex_server.get_user(&req);

    info!("user {user}: POST /indexes/{index_id}/guarded_write");

    check_permission(&user, &index_id, Permission::Write, &findex_server).await?;

    let error_prefix: String =
        format!("Invalid {OPERATION_NAME} request by {user} on index {index_id}.");

    let discriminant_flag = bytes.get(ADDRESS_LENGTH); // 0 or 1. 0 means None, 1 means Some. Assumes the first ADDRESS_LENGTH bytes are the address

    let flag = if let Some(f) = discriminant_flag {
        match *f {
            0 => false,
            1 => true,
            invalid => {
                return Err(FindexServerError::InvalidRequest(format!(
                    "{error_prefix} Invalid discriminant flag. Expected 0 or 1, found {invalid}"
                )))
            }
        }
    } else {
        return Err(FindexServerError::InvalidRequest(format!(
            "{error_prefix} Invalid discriminant flag. Expected 0 or 1, found None"
        )));
    };

    let guard_len = if flag {
        ADDRESS_LENGTH + 1 + WORD_LENGTH
    } else {
        ADDRESS_LENGTH + 1
    };

    let guard = bytes.get(..guard_len);
    if guard.is_none() {
        return Err(FindexServerError::InvalidRequest(format!(
            "{error_prefix} Could not parse guard.",
        )));
    }
    let tasks = bytes.get(guard_len..);
    if tasks.is_none() {
        return Err(FindexServerError::InvalidRequest(format!(
            "{error_prefix} Could not parse tasks to be written.",
        )));
    }

    #[allow(clippy::unwrap_used)] // guard and tasks are checked to be Some just above
    let guard = Guard::deserialize(guard.unwrap())?;
    #[allow(clippy::unwrap_used)] // same as above, already checked to be Some
    let tasks = Tasks::deserialize(tasks.unwrap())?;

    let result_word = findex_server
        .db
        .guarded_write(guard.clone().into_inner(), tasks.clone().into_inner())
        .await?;

    trace!(
        "{}",
        if result_word == guard.clone().into_inner().1 {
            format!(
                "Guarded_write SUCCESS. Expected guard value matched. {} words written.",
                tasks.into_inner().len()
            )
        } else {
            format!(
                "Guarded_write FAILED. Guard mismatch: expected={guard:?}, found={result_word:?}",
            )
        }
    );

    let response_bytes = Bytes::from(OptionalWords::new(vec![result_word]).serialize()?);

    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(response_bytes))
}
