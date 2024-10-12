use actix_web::{HttpMessage, HttpRequest};
use tracing::debug;

use crate::{
    config::ServerParams,
    database::Database,
    middlewares::{JwtAuthClaim, PeerCommonName},
};

/// A Simple Key Management System that partially implements KMIP 2.1:
/// `https://www.oasis-open.org/committees/tc_home.php?wg_abbrev=kmip`
#[allow(dead_code)]
pub(crate) struct FindexServer {
    pub(crate) params: ServerParams,
    pub(crate) db: Box<dyn Database + Sync + Send>,
}

/// Implement the KMIP Server operations and dispatches the actual actions
/// to the implementation module or ciphers for encryption/decryption
impl FindexServer {
    /// Get the user from the request depending on the authentication method
    /// The user is encoded in the JWT `Authorization` header
    /// If the header is not present, the user is extracted from the client certificate
    /// If the client certificate is not present, the user is extracted from the configuration file
    pub(crate) fn get_user(&self, req_http: &HttpRequest) -> String {
        let default_username = self.params.default_username.clone();

        if self.params.force_default_username {
            debug!(
                "Authenticated using forced default user: {}",
                default_username
            );
            return default_username;
        }
        // if there is a JWT token, use it in priority
        let user = req_http.extensions().get::<JwtAuthClaim>().map_or_else(
            || {
                req_http
                    .extensions()
                    .get::<PeerCommonName>()
                    .map_or(default_username, |claim| claim.common_name.clone())
            },
            |claim| claim.email.clone(),
        );
        debug!("Authenticated user: {}", user);
        user
    }
}
