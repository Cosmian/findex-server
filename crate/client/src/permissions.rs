use cosmian_findex_structs::{Permission, Permissions};
use tracing::{instrument, trace};
use uuid::Uuid;

use crate::{
    error::{result::FindexClientResult, FindexClientError},
    handle_error,
    rest_client::SuccessResponse,
    FindexRestClient,
};

impl FindexRestClient {
    #[instrument(ret(Display), err, skip(self))]
    pub async fn create_index_id(&self) -> FindexClientResult<SuccessResponse> {
        let endpoint = "/create/index".to_owned();
        let server_url = format!("{}{endpoint}", self.client.server_url);
        trace!("POST: {server_url}");
        let response = self.client.client.post(server_url).send().await?;
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
    pub async fn grant_permission(
        &self,
        user_id: &str,
        permission: &Permission,
        index_id: &Uuid,
    ) -> FindexClientResult<SuccessResponse> {
        let endpoint = format!("/permission/grant/{user_id}/{permission}/{index_id}");
        let server_url = format!("{}{endpoint}", self.client.server_url);
        trace!("POST: {server_url}");
        let response = self.client.client.post(server_url).send().await?;
        let status_code = response.status();
        if status_code.is_success() {
            return Ok(response.json::<SuccessResponse>().await?);
        }

        // process error
        let p = handle_error(&endpoint, response).await?;
        Err(FindexClientError::RequestFailed(p))
    }

    #[instrument(ret(Display), err, skip(self))]
    pub async fn list_permission(&self, user_id: &str) -> FindexClientResult<Permissions> {
        let endpoint = format!("/permission/list/{user_id}");
        let server_url = format!("{}{endpoint}", self.client.server_url);
        trace!("POST: {server_url}");
        let response = self.client.client.post(server_url).send().await?;
        let status_code = response.status();
        if status_code.is_success() {
            let response_bytes = response.bytes().await.map(|r| r.to_vec())?;
            let permissions = Permissions::deserialize(&response_bytes)?;
            return Ok(permissions);
        }

        // process error
        let p = handle_error(&endpoint, response).await?;
        Err(FindexClientError::RequestFailed(p))
    }

    #[instrument(ret(Display), err, skip(self))]
    pub async fn revoke_permission(
        &self,
        user_id: &str,
        index_id: &Uuid,
    ) -> FindexClientResult<SuccessResponse> {
        let endpoint = format!("/permission/revoke/{user_id}/{index_id}");
        let server_url = format!("{}{endpoint}", self.client.server_url);
        trace!("POST: {server_url}");
        let response = self.client.client.post(server_url).send().await?;
        let status_code = response.status();
        if status_code.is_success() {
            return Ok(response.json::<SuccessResponse>().await?);
        }

        // process error
        let p = handle_error(&endpoint, response).await?;
        Err(FindexClientError::RequestFailed(p))
    }
}
