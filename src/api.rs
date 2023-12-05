use std::sync::Mutex;

use actix_web::{
    post,
    web::{Data, Json},
};
use cloudproof_findex::InstantiatedFindex;

#[post("/v1/fetch")]
pub async fn fetch(body: Json<String>, findex: Data<Mutex<InstantiatedFindex>>) -> Json<String> {
    println!("Received: {body}");
    Json("OK".to_string())
}
