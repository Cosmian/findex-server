use cosmian_findex_client::FindexRestClient;
use cosmian_findex_structs::Permission;
use uuid::Uuid;

use crate::{
    actions::permissions::{CreateIndex, GrantPermission, ListPermissions, RevokePermission},
    error::result::CliResult,
};

pub(crate) async fn create_index_id(rest_client: &mut FindexRestClient) -> CliResult<Uuid> {
    Ok(CreateIndex.run(rest_client).await?)
}

pub(crate) async fn list_permission(
    rest_client: &mut FindexRestClient,
    user: String,
) -> CliResult<String> {
    Ok(ListPermissions { user }.run(rest_client).await?)
}

pub(crate) async fn grant_permission(
    rest_client: &mut FindexRestClient,
    user: String,
    index_id: &Uuid,
    permission: Permission,
) -> CliResult<String> {
    Ok(GrantPermission {
        user,
        index_id: *index_id,
        permission,
    }
    .run(rest_client)
    .await?
    .to_string())
}

pub(crate) async fn revoke_permission(
    rest_client: &mut FindexRestClient,
    user: String,
    index_id: &Uuid,
) -> CliResult<String> {
    Ok(RevokePermission {
        user,
        index_id: *index_id,
    }
    .run(rest_client)
    .await?)
}
