use cosmian_findex_structs::{EncryptedEntries, Uuids};
use tracing::{instrument, trace};
use uuid::Uuid;

use crate::{
    error::{result::FindexClientResult, FindexClientError},
    findex_rest_client::SuccessResponse,
    handle_error, FindexClient,
};

impl FindexClient {
    // #[instrument(ret(Display), err, skip(self))]
    pub async fn add_entries(
        // todo(manu): revisit function names (prefix with dataset_, findex_, permissions)
        &self,
        index_id: &Uuid,
        encrypted_entries: &EncryptedEntries,
    ) -> FindexClientResult<SuccessResponse> {
        let endpoint = format!("/datasets/{index_id}/add_entries");
        let server_url = format!("{}{endpoint}", self.client.server_url);
        trace!("POST: {server_url}");
        let encrypted_entries = encrypted_entries.serialize()?;
        let response = self
            .client
            .client
            .post(server_url)
            .body(encrypted_entries)
            .send()
            .await?;

        trace!("Response: {response:?}");
        let status_code = response.status();
        if status_code.is_success() {
            return Ok(response.json::<SuccessResponse>().await?);
        }

        // process error
        let p = handle_error(&endpoint, response).await?;
        Err(FindexClientError::RequestFailed(p))
    }

    #[instrument(ret(Display), err, skip(self))]
    pub async fn delete_entries(
        &self,
        index_id: &Uuid,
        uuids: &Uuids,
    ) -> FindexClientResult<SuccessResponse> {
        let endpoint = format!("/datasets/{index_id}/delete_entries");
        let server_url = format!("{}{endpoint}", self.client.server_url);
        trace!("POST: {server_url}");

        let uuids = uuids.serialize();
        let response = self
            .client
            .client
            .post(server_url)
            .body(uuids)
            .send()
            .await?;
        let status_code = response.status();
        if status_code.is_success() {
            return Ok(response.json::<SuccessResponse>().await?);
        }

        // process error
        let p = handle_error(&endpoint, response).await?;
        Err(FindexClientError::RequestFailed(p))
    }

    #[instrument(ret(Display), err, skip(self))]
    pub async fn get_entries(
        &self,
        index_id: &Uuid,
        uuids: &Uuids,
    ) -> FindexClientResult<EncryptedEntries> {
        let endpoint = format!("/datasets/{index_id}/get_entries");
        let server_url = format!("{}{endpoint}", self.client.server_url);
        trace!("POST: {server_url}");

        let uuids = uuids.serialize();
        let response = self
            .client
            .client
            .post(server_url)
            .body(uuids)
            .send()
            .await?;
        let status_code = response.status();
        if status_code.is_success() {
            let response_bytes = response.bytes().await.map(|r| r.to_vec())?;
            let encrypted_entries = EncryptedEntries::deserialize(&response_bytes)?;
            return Ok(encrypted_entries);
        }

        // process error
        let p = handle_error(&endpoint, response).await?;
        Err(FindexClientError::RequestFailed(p))
    }
}
