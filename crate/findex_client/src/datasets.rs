use cosmian_findex_structs::{EncryptedEntries, Uuids};
use cosmian_kms_cli::reexport::cosmian_kms_crypto::reexport::cosmian_crypto_core::bytes_ser_de::Serializable;
use tracing::{instrument, trace};
use uuid::Uuid;

use crate::{
    RestClient,
    error::{ClientError, result::ClientResult},
    rest_client::{SuccessResponse, handle_error, handle_status_code},
};

impl RestClient {
    /// Add encrypted entries to a dataset.
    #[instrument(ret(Display), err, skip_all, level = "trace")]
    pub async fn add_entries(
        &self,
        index_id: &Uuid,
        encrypted_entries: &EncryptedEntries,
    ) -> ClientResult<SuccessResponse> {
        let endpoint = format!("/datasets/{index_id}/add_entries");
        let server_url = format!("{}{endpoint}", self.http_client.server_url);
        trace!("POST: {server_url}");
        let encrypted_entries = encrypted_entries.serialize()?;
        let response = self
            .http_client
            .client
            .post(server_url)
            .body(encrypted_entries.to_vec())
            .send()
            .await?;

        handle_status_code(response, &endpoint).await
    }

    /// Delete entries from a dataset using their UUIDs.
    #[instrument(ret(Display), err, skip(self), level = "trace")]
    pub async fn delete_entries(
        &self,
        index_id: &Uuid,
        uuids: &[Uuid],
    ) -> ClientResult<SuccessResponse> {
        let endpoint = format!("/datasets/{index_id}/delete_entries");
        let server_url = format!("{}{endpoint}", self.http_client.server_url);
        trace!("POST: {server_url}");

        let uuids = Uuids::from(uuids).serialize()?;
        let response = self
            .http_client
            .client
            .post(server_url)
            .body(uuids.to_vec())
            .send()
            .await?;

        handle_status_code(response, &endpoint).await
    }

    /// Get entries from a dataset using their UUIDs.
    #[instrument(ret(Display), err, skip(self), level = "trace")]
    pub async fn get_entries(
        &self,
        index_id: &Uuid,
        uuids: &[Uuid],
    ) -> ClientResult<EncryptedEntries> {
        let endpoint = format!("/datasets/{index_id}/get_entries");
        let server_url = format!("{}{endpoint}", self.http_client.server_url);
        trace!("POST: {server_url}");

        let uuids = Uuids::from(uuids).serialize()?;
        let response = self
            .http_client
            .client
            .post(server_url)
            .body(uuids.to_vec())
            .send()
            .await?;
        if response.status().is_success() {
            let response_bytes = response.bytes().await.map(|r| r.to_vec())?;
            let encrypted_entries = EncryptedEntries::deserialize(&response_bytes)?;
            return Ok(encrypted_entries);
        }

        Err(ClientError::RequestFailed(
            handle_error(&endpoint, response).await?,
        ))
    }
}
