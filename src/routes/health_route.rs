use actix_web::{HttpResponse, Responder};

use crate::services::health;

pub(crate) async fn health_get() -> impl Responder {
    if health() {
        return HttpResponse::Ok().json(serde_json::json!({"status": "OK"}))
    }
    HttpResponse::ServiceUnavailable().json("Service Unhealthy")
}

// check this out:
// https://stackoverflow.com/questions/74816014/passing-actix-web-payload-to-function
