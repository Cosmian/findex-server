use std::sync::Arc;

use actix_web::{
    post,
    web::{self, Bytes, Data},
    HttpRequest, HttpResponse,
};
use cosmian_findex::{MemoryADT, ADDRESS_LENGTH};
use cosmian_findex_structs::Permission;
use tracing::{info, trace};
use uuid::Uuid;

use crate::{
    core::FindexServer,
    database::redis::{Redis, WORD_LENGTH},
    routes::{check_permission, error::ResponseBytes},
};

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

    type AddressType = <Redis<WORD_LENGTH> as MemoryADT>::Address; // keeps a SSOT for types

    let bytes_slice = bytes.as_ref();
    assert!(
        bytes_slice.len() % ADDRESS_LENGTH == 0,
        "Bytes length must be multiple of address size"
    );

    trace!(
        "batch_read: number of addresses {}:",
        bytes_slice.len() / ADDRESS_LENGTH
    );

    // Collect into a vector to adhere the memory interface
    let addresses: Vec<AddressType> = bytes_slice
        .chunks_exact(ADDRESS_LENGTH)
        .map(|chunk| {
            let array: [u8; ADDRESS_LENGTH] = chunk
                .try_into()
                .expect("Chunk size guaranteed by chunks_exact, this should not fail.");
            AddressType::from(array)
        })
        .collect();

    let result_words = findex_server.db.batch_read(addresses).await?;
    trace!(
        "batch_read: number of non null words: {}:",
        result_words
            .iter()
            .fold(0, |acc, x| acc + (x.is_some() as usize))
    );

    // Convert Vec<Option<[u8; WORD_LENGTH]>> to Vec<u8>
    let response_bytes = Bytes::from(
        result_words
            .into_iter()
            .flat_map(|opt_word| {
                let mut bytes = vec![0u8; 1]; // Option discriminant byte
                match opt_word {
                    Some(word) => {
                        bytes[0] = 1;
                        bytes.extend_from_slice(&<[u8; WORD_LENGTH]>::from(word));
                    }
                    None => {
                        bytes[0] = 0;
                    }
                }
                bytes
            })
            .collect::<Vec<u8>>(),
    );

    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(Bytes::from(response_bytes)))
}

#[post("/indexes/{index_id}/guarded_write")]
pub(crate) async fn findex_guarded_write(
    req: HttpRequest,
    index_id: web::Path<String>,
    bytes: Bytes,
    findex_server: Data<Arc<FindexServer>>,
) -> ResponseBytes {
    let user = findex_server.get_user(&req);
    info!("user {user}: POST /indexes/{index_id}/guarded_write");

    check_permission(&user, &index_id, Permission::Read, &findex_server).await?;

    type AddressType = <Redis<WORD_LENGTH> as MemoryADT>::Address;
    type WordType = <Redis<WORD_LENGTH> as MemoryADT>::Word; // same as above, keeping SSOT for words typing

    let bytes_slice = bytes.as_ref();

    // Size calculations to assert the byte stream is valid for guarded_write operation requirements
    const ADDRESS_SIZE: usize = std::mem::size_of::<AddressType>();
    const WORD_SIZE: usize = std::mem::size_of::<WordType>();
    const GUARD_SIZE: usize = ADDRESS_SIZE + (1 + WORD_SIZE); // +1 for Option discriminant

    // Assert total length
    assert!(
        bytes_slice.len() >= GUARD_SIZE,
        "Byte stream too short for guard structure"
    );

    // Assert remaining bytes are valid (adr/word) pairs
    let tasks_bytes = &bytes_slice[GUARD_SIZE..];
    assert!(
        tasks_bytes.len() % (ADDRESS_SIZE + WORD_SIZE) == 0,
        "tasks payload must be multiple of (address,word) pairs"
    );

    let task_count = tasks_bytes.len() / (ADDRESS_SIZE + WORD_SIZE);
    trace!("Guarded_write called for {} tasks", task_count);

    let guard: (AddressType, Option<WordType>) = {
        let address = AddressType::from(
            <[u8; ADDRESS_LENGTH]>::try_from(&bytes_slice[..ADDRESS_SIZE])
                .expect("Slice length guaranteed by previous checks"),
        );

        let word = if bytes_slice[ADDRESS_SIZE] != 0 {
            Some(WordType::from(
                <[u8; WORD_LENGTH]>::try_from(&bytes_slice[ADDRESS_SIZE + 1..GUARD_SIZE])
                    .expect("Slice length guaranteed by previous checks"),
            ))
        } else {
            None
        };
        (address, word)
    };

    let tasks: Vec<(AddressType, WordType)> = bytes_slice[..GUARD_SIZE]
        .chunks_exact(ADDRESS_LENGTH + WORD_LENGTH)
        .map(|chunk| {
            let (addr_bytes, word_bytes) = chunk.split_at(ADDRESS_SIZE);
            // Convert address
            let address = AddressType::from(
                <[u8; ADDRESS_LENGTH]>::try_from(addr_bytes)
                    .expect("Chunk size guaranteed by chunks_exact"),
            );
            // Convert word
            let word = WordType::from(
                <[u8; WORD_LENGTH]>::try_from(word_bytes)
                    .expect("Chunk size guaranteed by chunks_exact"),
            );

            (address, word)
        })
        .collect();

    // TODO(hatem): can probably avoid cloning here
    let result_word = findex_server
        .db
        .guarded_write(guard.clone(), tasks.clone())
        .await?;

    if result_word == guard.1 {
        format!(
            "Guarded_write SUCCESS. Expected guard value matched. {} words written.",
            tasks.len()
        )
    } else {
        format!(
            "Guarded_write FAILED. Guard mismatch: expected={:?}, found={:?}",
            guard, result_word
        )
    };

    let response_bytes = Bytes::from({
        let mut bytes = vec![0u8; 1];
        match result_word {
            Some(word) => {
                bytes[0] = 1;
                bytes.extend_from_slice(&word);
            }
            None => {
                bytes[0] = 0;
            }
        };
        bytes
    });

    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(response_bytes))
}
