// use crate::findex_backend::SqliteFindexBackend;
use actix_web::{post, web::Json, HttpMessage, HttpRequest, Responder};
use log::{info, log};

pub async fn health(_req: HttpRequest) -> impl Responder {
    info!("Health check !");

    Json("\nOK".to_string())
}
