use cosmian_findex_client::FindexRestClient;
use cosmian_findex_structs::Permission;
use uuid::Uuid;

use crate::{
    actions::permissions::{CreateIndex, ListPermissions, RevokePermission, SetPermission},
    error::result::CliResult,
};

pub(crate) async fn create_index_id(rest_client: &FindexRestClient) -> CliResult<Uuid> {
    CreateIndex.run(rest_client).await
}

pub(crate) async fn list_permission(
    rest_client: &FindexRestClient,
    user: String,
) -> CliResult<String> {
    ListPermissions { user }.run(rest_client).await
}

pub(crate) async fn set_permission(
    rest_client: &FindexRestClient,
    user: String,
    index_id: Uuid,
    permission: Permission,
) -> CliResult<String> {
    SetPermission {
        user,
        index_id,
        permission,
    }
    .run(rest_client)
    .await
}

pub(crate) async fn revoke_permission(
    rest_client: &FindexRestClient,
    user: String,
    index_id: Uuid,
) -> CliResult<String> {
    RevokePermission { user, index_id }.run(rest_client).await
}
