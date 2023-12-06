use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};

use crate::findex_backend::SqliteFindexBackend;
use actix_web::{
    post,
    web::{Data, Json},
};
use cosmian_findex::{EdxBackend, TokenToEncryptedValueMap, Tokens};

/// `curl -X POST localhost:8080/v1/fetch  -H 'Content-Type: application/json' -d '"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"'`
#[post("/v1/fetch")]
pub async fn fetch(
    body: Json<String>,
    findex_backend: Data<Mutex<SqliteFindexBackend>>,
) -> Json<String> {
    println!("Received: {body}, {}", body.len());
    let token: [u8; 32] = body.as_bytes().try_into().unwrap();
    let mut tokens = HashSet::with_capacity(1);
    tokens.insert(token.into());

    let res = findex_backend
        .lock()
        .unwrap()
        .entry
        .fetch(Tokens(tokens))
        .await
        .unwrap();

    println!("Output: {res}");
    Json("OK".to_string())
}

/// `curl -X POST localhost:8080/v1/insert -H 'Content-Type: application/json' -d '"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"'`
#[post("/v1/insert")]
pub async fn insert(
    body: Json<String>,
    findex_backend: Data<Mutex<SqliteFindexBackend>>,
) -> Json<String> {
    println!("Received: {body}, {}", body.len());
    let token: [u8; 32] = body.as_bytes().try_into().unwrap();
    // random value to test
    let encrypted_value = [65u8; 80 + 16 + 12];
    let mut tokens_ev_map = HashMap::with_capacity(1);
    tokens_ev_map.insert(token.into(), encrypted_value.as_slice().try_into().unwrap());

    findex_backend
        .lock()
        .unwrap()
        .entry
        .insert(TokenToEncryptedValueMap(tokens_ev_map))
        .await
        .unwrap();

    Json("OK".to_string())
}
