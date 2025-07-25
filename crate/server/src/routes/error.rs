use actix_web::{
    HttpResponse, HttpResponseBuilder,
    http::{StatusCode, header},
};
use serde::{Deserialize, Serialize};
use tracing::{error, warn};
use uuid::Uuid;

use crate::error::server::ServerError;

pub(crate) type ResponseBytes = Result<HttpResponse, ServerError>;

impl actix_web::error::ResponseError for ServerError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,

            Self::DatabaseError(_)
            | Self::ConversionError(_)
            | Self::CryptographicError(_)
            | Self::Findex(_)
            | Self::SendError(_)
            | Self::Certificate(_)
            | Self::StructsError(_)
            | Self::OpenSslError(_)
            | Self::UuidError(_)
            | Self::ServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,

            Self::InvalidRequest(_) | Self::ClientConnectionError(_) | Self::UrlParseError(_) => {
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

#[derive(Deserialize, Serialize, Debug)] // Debug is required by ok_json()
pub(crate) struct SuccessResponse {
    pub success: String,
    pub index_id: Uuid,
}
