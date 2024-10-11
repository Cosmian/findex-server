use std::sync::Arc;

use alcoholic_jwt::token_kid;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::middlewares::error::LoginError;

use super::{jwks::JwksManager, types::LoginResult};

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct UserClaim {
    pub email: Option<String>,
    pub iss: Option<String>,
    pub sub: Option<String>,
    pub aud: Option<Vec<String>>,
    pub iat: Option<usize>,
    pub exp: Option<usize>,
    pub nbf: Option<usize>,
    pub jti: Option<String>,
    pub scope: Option<String>,
    // Google CSE
    pub role: Option<String>,
    // Google CSE
    pub resource_name: Option<String>,
    // Google CSE
    pub perimeter_id: Option<String>,
    // Google CSE
    pub kacls_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct JwtTokenHeaders {
    typ: Option<String>,
    cty: Option<String>,
    alg: Option<String>,
    kid: Option<String>,
    x5t: Option<String>,
    x5u: Option<String>,
    x5c: Option<Vec<String>>,
    crit: Option<String>,
}

#[derive(Debug)]
pub struct JwtConfig {
    pub jwt_issuer_uri: String,
    pub jwt_audience: Option<String>,
    pub jwks: Arc<JwksManager>,
}

impl JwtConfig {
    /// Decode a JWT bearer header
    pub(crate) fn decode_bearer_header(
        &self,
        authorization_content: &str,
    ) -> LoginResult<UserClaim> {
        let bearer: Vec<&str> = authorization_content.splitn(2, ' ').collect();
        // assert!(
        //     bearer.len() == 2 && bearer[0] == "Bearer",
        //     "bad auth header content"
        // );
        // LoginError::Unauthorized(("Bad authorization header content (bad bearer)".to_owned()})
        if !(bearer.len() == 2 && bearer[0] == "Bearer") {
            return Err(LoginError::Unauthorized(
                "Bad authorization header content (bad bearer)".to_owned(),
            ));
        }

        let token: &str = bearer[1];
        self.decode_authentication_token(token)
    }

    /// Decode a json web token (JWT)
    pub(crate) fn decode_authentication_token(&self, token: &str) -> LoginResult<UserClaim> {
        assert!(
            !token.is_empty(),
            "token is empty" // LoginError::Unauthorized(("token is empty".to_owned()})
        );
        tracing::trace!(
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
            validations.push(alcoholic_jwt::Validation::Audience(
                jwt_audience.to_string(),
            ));
        }

        // If a JWKS contains multiple keys, the correct KID first
        // needs to be fetched from the token headers.
        let kid = token_kid(token)
            .map_err(|e| LoginError::Unauthorized(format!("Failed to decode kid: {e}")))?
            .ok_or_else(|| {
                LoginError::Unauthorized("No 'kid' claim present in token".to_owned())
            })?;

        tracing::trace!("looking for kid `{kid}` JWKS:\n{:?}", self.jwks);

        let jwk = self
            .jwks
            .find(&kid)?
            .ok_or_else(|| LoginError::Unauthorized("Specified key not found in set".to_owned()))?;

        tracing::trace!("JWK has been found:\n{jwk:?}");

        let valid_jwt = alcoholic_jwt::validate(token, &jwk, validations).map_err(|err| {
            LoginError::Unauthorized(("Cannot validate token: {err:?}".to_owned()))
        })?;

        debug!("JWT is valid, the claims are {0:?}", valid_jwt.claims);

        let payload = serde_json::from_value(valid_jwt.claims).map_err(|err| {
            LoginError::Unauthorized(("JWT claims is malformed: {err:?}".to_owned()))
        })?;

        debug!("JWT payload: {payload:?}");

        Ok(payload)
    }
}
