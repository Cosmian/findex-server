use actix_web::{HttpRequest, HttpResponse, Responder};

use crate::services::health;

pub async fn health_get(_req: HttpRequest) -> impl Responder {
    let is_health_ok: bool = health();
    match is_health_ok {
        true => HttpResponse::Ok().json(serde_json::json!({"status": "OK"})),
        false => HttpResponse::ServiceUnavailable().json("Service Unhealthy"),
    }
}
