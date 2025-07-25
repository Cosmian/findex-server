use cosmian_findex_structs::{Permission, Permissions};
use cosmian_kms_cli::reexport::cosmian_kms_crypto::reexport::cosmian_crypto_core::bytes_ser_de::Serializable;
use tracing::{instrument, trace};
use uuid::Uuid;

use crate::{
    ClientError, ClientResult, RestClient,
    rest_client::{SuccessResponse, handle_error, handle_status_code},
};

impl RestClient {
    /// Create a new index ID.
    #[instrument(ret(Display), err, skip(self), level = "trace")]
    pub async fn create_index_id(&self) -> ClientResult<SuccessResponse> {
        let endpoint = "/create/index";
        let server_url = format!("{}{endpoint}", self.http_client.server_url);
        trace!("POST: {server_url}");
        let response = self.http_client.client.post(server_url).send().await?;

        handle_status_code(response, endpoint).await
    }

    /// Set a permission for a user on an index.
    #[instrument(ret(Display), err, skip(self), level = "trace")]
    pub async fn set_permission(
        &self,
        user_id: &str,
        permission: &Permission,
        index_id: &Uuid,
    ) -> ClientResult<SuccessResponse> {
        let endpoint = format!("/permission/set/{user_id}/{permission}/{index_id}");
        let server_url = format!("{}{endpoint}", self.http_client.server_url);
        trace!("POST: {server_url}");
        let response = self.http_client.client.post(server_url).send().await?;

        handle_status_code(response, &endpoint).await
    }

    /// List all permissions for a user.
    #[instrument(ret(Display), err, skip(self), level = "trace")]
    pub async fn list_permission(&self, user_id: &str) -> ClientResult<Permissions> {
        let endpoint = format!("/permission/list/{user_id}");
        let server_url = format!("{}{endpoint}", self.http_client.server_url);
        trace!("POST: {server_url}");
        let response = self.http_client.client.post(server_url).send().await?;
        if response.status().is_success() {
            let response_bytes = response.bytes().await.map(|r| r.to_vec())?;
            let permissions = Permissions::deserialize(&response_bytes)?;
            return Ok(permissions);
        }

        Err(ClientError::RequestFailed(
            handle_error(&endpoint, response).await?,
        ))
    }

    /// Revoke a permission for a user on an index.
    #[instrument(ret(Display), err, skip(self), level = "trace")]
    pub async fn revoke_permission(
        &self,
        user_id: &str,
        index_id: &Uuid,
    ) -> ClientResult<SuccessResponse> {
        let endpoint = format!("/permission/revoke/{user_id}/{index_id}");
        let server_url = format!("{}{endpoint}", self.http_client.server_url);
        trace!("POST: {server_url}");
        let response = self.http_client.client.post(server_url).send().await?;

        handle_status_code(response, &endpoint).await
    }
}
