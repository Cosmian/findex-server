use std::{rc::Rc, sync::Arc};

use actix_identity::Identity;
use actix_service::Service;
use actix_web::{
    Error, FromRequest, HttpMessage, HttpResponse,
    body::{BoxBody, EitherBody},
    dev::{ServiceRequest, ServiceResponse},
    http::header,
};
use tracing::{debug, error, trace};

use super::UserClaim;
use crate::{
    error::{result::FResult, server::ServerError},
    middlewares::jwt::JwtConfig,
};

pub(crate) async fn manage_jwt_request<S, B>(
    service: Rc<S>,
    configs: Arc<Vec<JwtConfig>>,
    req: ServiceRequest,
) -> Result<ServiceResponse<EitherBody<B, BoxBody>>, Error>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
{
    trace!("Starting JWT Authentication...");
    match manage_jwt(configs, &req).await {
        Ok(auth_claim) => {
            req.extensions_mut().insert(auth_claim);
            Ok(service.call(req).await?.map_into_left_body())
        }
        Err(e) => {
            error!("{:?} {} 401 unauthorized: {e:?}", req.method(), req.path(),);
            Ok(req
                .into_response(HttpResponse::Unauthorized().finish())
                .map_into_right_body())
        }
    }
}

fn extract_user_claim(
    configs: &[JwtConfig],
    identity: &str,
) -> Result<UserClaim, Vec<ServerError>> {
    let mut jwt_log_errors = Vec::new();
    for idp_config in configs {
        match idp_config.decode_bearer_header(identity) {
            Ok(user_claim) => return Ok(user_claim),
            Err(error) => {
                jwt_log_errors.push(error);
            }
        }
    }
    Err(jwt_log_errors)
}

pub(crate) async fn manage_jwt(
    configs: Arc<Vec<JwtConfig>>,
    req: &ServiceRequest,
) -> FResult<JwtAuthClaim> {
    trace!("JWT Authentication...");

    let identity = Identity::extract(req.request())
        .into_inner()
        .map_or_else(
            |_| {
                req.headers()
                    .get(header::AUTHORIZATION)
                    .and_then(|h| h.to_str().ok().map(std::string::ToString::to_string))
            },
            |identity| identity.id().ok(),
        )
        .unwrap_or_default();

    trace!("Checking JWT identity: {identity}");

    let mut private_claim = extract_user_claim(&configs, &identity);
    // If no configuration could get the claim, try refreshing them and extract user
    // claim again
    if private_claim.is_err() {
        configs
            .first()
            .ok_or_else(|| ServerError::ServerError("No config available".to_owned()))?
            .jwks
            .refresh()
            .await?;
        private_claim = extract_user_claim(&configs, &identity);
    }

    match private_claim.map(|user_claim| user_claim.email) {
        Ok(Some(email)) => {
            debug!("JWT Access granted to {email}!");
            Ok(JwtAuthClaim::new(email))
        }
        Ok(None) => {
            error!(
                "{:?} {} 401 unauthorized, no email in JWT",
                req.method(),
                req.path()
            );
            Err(ServerError::InvalidRequest("No email in JWT".to_owned()))
        }
        Err(jwt_log_errors) => {
            for error in &jwt_log_errors {
                tracing::error!("{error:?}");
            }
            error!(
                "{:?} {} 401 unauthorized: bad JWT",
                req.method(),
                req.path(),
            );
            Err(ServerError::InvalidRequest("bad JWT".to_owned()))
        }
    }
}

#[derive(Debug)]
pub(crate) struct JwtAuthClaim {
    pub email: String,
}

impl JwtAuthClaim {
    #[must_use]
    pub(crate) const fn new(email: String) -> Self {
        Self { email }
    }
}
