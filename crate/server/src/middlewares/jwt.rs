use std::sync::Arc;

use alcoholic_jwt::token_kid;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace};

use super::JwksManager;
use crate::{
    error::{result::FResult, server::ServerError},
    findex_server_ensure,
};

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct UserClaim {
    pub email: Option<String>,
    pub iss: Option<String>,
    pub sub: Option<String>,
    pub aud: Option<String>,
    pub iat: Option<usize>,
    pub exp: Option<usize>,
    pub nbf: Option<usize>,
    pub jti: Option<String>,
    // Google CSE
    pub role: Option<String>,
    // Google CSE
    pub resource_name: Option<String>,
    // Google CSE
    pub perimeter_id: Option<String>,
    // Google CSE
    pub kacls_url: Option<String>,
    // Google CSE
    pub spki_hash: Option<String>,
    // Google CSE
    pub spki_hash_algorithm: Option<String>,
    // Google CSE
    pub message_id: Option<String>,
    // Google CSE
    pub email_type: Option<String>,
    // Google CSE
    pub google_email: Option<String>,
}

#[derive(Debug)]
pub(crate) struct JwtConfig {
    pub jwt_issuer_uri: String,
    pub jwt_audience: Option<String>,
    pub jwks: Arc<JwksManager>,
}

impl JwtConfig {
    /// Decode a JWT bearer header
    pub(crate) fn decode_bearer_header(&self, authorization_content: &str) -> FResult<UserClaim> {
        let bearer: Vec<&str> = authorization_content.splitn(2, ' ').collect();
        findex_server_ensure!(
            bearer.first().ok_or_else(|| ServerError::Unauthorized(
                "Bad authorization header content (missing bearer)".to_owned()
            ))? == &"Bearer"
                && bearer.get(1).is_some(),
            ServerError::Unauthorized("Bad authorization header content (bad bearer)".to_owned())
        );

        let token: &str = bearer.get(1).ok_or_else(|| {
            ServerError::Unauthorized("Bad authorization header content (missing token)".to_owned())
        })?;
        self.decode_authentication_token(token)
    }

    /// Decode a json web token (JWT)
    #[allow(clippy::cognitive_complexity)]
    pub(crate) fn decode_authentication_token(&self, token: &str) -> FResult<UserClaim> {
        findex_server_ensure!(
            !token.is_empty(),
            ServerError::Unauthorized("token is empty".to_owned())
        );
        trace!(
            "validating authentication token, expected JWT issuer: {}",
            self.jwt_issuer_uri
        );

        let mut validations = vec![
            #[cfg(not(test))]
            alcoholic_jwt::Validation::Issuer(self.jwt_issuer_uri.clone()),
            alcoholic_jwt::Validation::SubjectPresent,
            #[cfg(not(feature = "insecure"))]
            alcoholic_jwt::Validation::NotExpired,
        ];
        if let Some(jwt_audience) = &self.jwt_audience {
            validations.push(alcoholic_jwt::Validation::Audience(jwt_audience.clone()));
        }

        // If a JWKS contains multiple keys, the correct KID first
        // needs to be fetched from the token headers.
        let kid = token_kid(token)
            .map_err(|e| ServerError::Unauthorized(format!("Failed to decode kid: {e}")))?
            .ok_or_else(|| {
                ServerError::Unauthorized("No 'kid' claim present in token".to_owned())
            })?;

        trace!("looking for kid `{kid}` JWKS:\n{:?}", self.jwks);

        let jwk = self.jwks.find(&kid)?.ok_or_else(|| {
            ServerError::Unauthorized("Specified key not found in set".to_owned())
        })?;

        trace!("JWK has been found:\n{jwk:?}");

        let valid_jwt = alcoholic_jwt::validate(token, &jwk, validations)
            .map_err(|err| ServerError::Unauthorized(format!("Cannot validate token: {err:?}")))?;

        let payload = serde_json::from_value(valid_jwt.claims).map_err(|err| {
            ServerError::Unauthorized(format!("JWT claims is malformed: {err:?}"))
        })?;

        debug!("JWT payload: {payload:?}");

        Ok(payload)
    }
}
