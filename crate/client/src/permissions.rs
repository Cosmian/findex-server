use cosmian_findex_structs::Permission;
use tracing::{instrument, trace};
use uuid::Uuid;

use crate::{
    error::{result::FindexClientResult, FindexClientError},
    findex_rest_client::SuccessResponse,
    handle_error, FindexClient,
};

impl FindexClient {
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
