use actix_web::{HttpMessage, HttpRequest};
use cosmian_findex_structs::{Permission, CUSTOM_WORD_LENGTH};
use tracing::{debug, trace};
use uuid::Uuid;

use crate::{
    config::{DbParams, ServerParams},
    database::{database_traits::PermissionsTrait, redis::Redis},
    error::{result::FResult, server::ServerError},
    middlewares::{JwtAuthClaim, PeerCommonName},
};
pub(crate) struct FindexServer {
    pub(crate) params: ServerParams,
    pub(crate) db: Redis<CUSTOM_WORD_LENGTH>,
}

impl FindexServer {
    pub(crate) async fn instantiate(mut shared_config: ServerParams) -> FResult<Self> {
        let db = match &mut shared_config.db_params {
            DbParams::Redis(url) => {
                Redis::instantiate(url.as_str(), shared_config.clear_db_on_start).await?
            }
        };

        Ok(Self {
            params: shared_config,
            db,
        })
    }

    /// Get the user from the request depending on the authentication method
    /// The user is encoded in the JWT `Authorization` header
    /// If the header is not present, the user is extracted from the client
    /// certificate.
    ///  If the client certificate is not present, the user is
    /// extracted from the configuration file
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
        trace!("Authenticated user: {}", user);
        user
    }

    pub(crate) async fn get_permission(
        &self,
        user_id: &str,
        index_id: &str,
    ) -> FResult<Permission> {
        if user_id == self.params.default_username {
            trace!("User is the default user and has admin access");
            return Ok(Permission::Admin);
        }

        // Parse index_id
        let index_id = Uuid::parse_str(index_id)?;

        let permission = self.db.get_permission(user_id, &index_id).await?;
        trace!("User {user_id} has: {permission}");
        Ok(permission)
    }

    pub(crate) async fn ensure_minimum_permission(
        &self,
        user: &str,
        index_id: &str,
        expected_permission: Permission,
    ) -> FResult<()> {
        let permission = self.get_permission(user, index_id).await?;
        trace!("ensure_minimum_permission: user {user} has permission {permission} on index {index_id}");
        if permission < expected_permission {
            return Err(ServerError::Unauthorized(format!(
                "User {user} with permission {permission} is not allowed to write on index {index_id}",
            )));
        }
        Ok(())
    }
}
