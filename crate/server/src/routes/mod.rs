use std::sync::Arc;

use actix_web::{
    get,
    http::{header, StatusCode},
    web::{Data, Json},
    HttpRequest, HttpResponse, HttpResponseBuilder,
};
use clap::crate_version;
use openssl::version;
use tracing::{error, info, warn};

use crate::{database::FServer, error::FindexServerError, result::FResult};

impl actix_web::error::ResponseError for FindexServerError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,

            Self::DatabaseError(_)
            | Self::ConversionError(_)
            | Self::CryptographicError(_)
            | Self::Redis(_)
            | Self::Findex(_)
            | Self::Certificate(_)
            | Self::ServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,

            Self::InvalidRequest(_) | Self::ClientConnectionError(_) | Self::UrlError(_) => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        let message = self.to_string();

        if status_code >= StatusCode::INTERNAL_SERVER_ERROR {
            error!("{status_code} - {message}");
        } else {
            warn!("{status_code} - {message}");
        }

        HttpResponseBuilder::new(status_code)
            .insert_header((header::CONTENT_TYPE, "text/html; charset=utf-8"))
            .body(message)
    }
}

/// Get the Findex server version
#[get("/version")]
pub(crate) async fn get_version(
    req: HttpRequest,
    findex_server: Data<Arc<FServer>>,
) -> FResult<Json<String>> {
    info!("GET /version {}", findex_server.get_user(&req));
    Ok(Json(format!(
        "{} ({})",
        crate_version!().to_owned(),
        version::version()
    )))
}
