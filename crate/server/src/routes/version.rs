use crate::{core::FindexServer, error::result::FResult};
use actix_web::{
    get,
    web::{Data, Json},
    HttpRequest,
};
use clap::crate_version;
use openssl::version;
use std::sync::Arc;
use tracing::info;

/// Get the Findex server version
#[get("/version")]
pub(crate) async fn get_version(
    req: HttpRequest,
    findex_server: Data<Arc<FindexServer>>,
) -> FResult<Json<String>> {
    info!("GET /version {}", findex_server.get_user(&req));
    Ok(Json(format!(
        "{} ({})",
        crate_version!().to_owned(),
        version::version()
    )))
}
