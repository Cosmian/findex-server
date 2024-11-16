use actix_web::{HttpMessage, HttpRequest};
use cosmian_findex_structs::Permission;
use tracing::{debug, instrument, trace};

use crate::{
    config::{DbParams, ServerParams},
    database::{DatabaseTraits, Redis},
    error::result::FResult,
    findex_server_bail,
    middlewares::{JwtAuthClaim, PeerCommonName},
    routes::get_index_id,
};

pub(crate) struct FindexServer {
    pub(crate) params: ServerParams,
    pub(crate) db: Box<dyn DatabaseTraits + Sync + Send>,
}

impl FindexServer {
    pub(crate) async fn instantiate(mut shared_config: ServerParams) -> FResult<Self> {
        let db: Box<dyn DatabaseTraits + Sync + Send> =
            if let Some(mut db_params) = shared_config.db_params.as_mut() {
                match &mut db_params {
                    DbParams::Redis(url) => Box::new(
                        Redis::instantiate(url.as_str(), shared_config.clear_db_on_start).await?,
                    ),
                }
            } else {
                findex_server_bail!("Fatal: no database configuration provided. Stopping.")
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
        debug!("Authenticated user: {}", user);
        user
    }

    #[instrument(ret(Display), err, skip(self))]
    pub(crate) async fn get_permission(
        &self,
        user_id: &str,
        index_id: &str,
    ) -> FResult<Permission> {
        if user_id == self.params.default_username {
            trace!("User is the default user and has admin access");
            return Ok(Permission::Admin);
        }

        let permission = self
            .db
            .get_permission(user_id, &get_index_id(index_id)?)
            .await?;
        trace!("User {user_id} has: {permission}");
        Ok(permission)
    }
}
