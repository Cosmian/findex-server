use async_trait::async_trait;
use cosmian_findex::{ADDRESS_LENGTH, Address, MemoryADT, MemoryError};
use cosmian_findex_structs::{EncryptedEntries, Permission, Permissions, Uuids, WORD_LENGTH};
use uuid::Uuid;

use crate::error::result::FResult;

#[async_trait]
pub(crate) trait PermissionsTrait: Sync + Send {
    //
    // Permissions
    //
    async fn create_index_id(&self, user_id: &str) -> FResult<Uuid>;
    async fn get_permissions(&self, user_id: &str) -> FResult<Permissions>;
    async fn get_permission(&self, user_id: &str, index_id: &Uuid) -> FResult<Permission>;
    async fn grant_permission(
        &self,
        user_id: &str,
        permission: Permission,
        index_id: &Uuid,
    ) -> FResult<()>;
    async fn revoke_permission(&self, user_id: &str, index_id: &Uuid) -> FResult<()>;
}

#[async_trait]
pub(crate) trait DatasetsTrait: Sync + Send {
    //
    // Dataset management
    //
    async fn dataset_add_entries(&self, index_id: &Uuid, entries: &EncryptedEntries)
    -> FResult<()>;
    async fn dataset_delete_entries(&self, index_id: &Uuid, uuids: &Uuids) -> FResult<()>;
    async fn dataset_get_entries(
        &self,
        index_id: &Uuid,
        uuids: &Uuids,
    ) -> FResult<EncryptedEntries>;
}

#[allow(dead_code)] // TODO : code is actually used, find why compiler thinks it's not
#[async_trait]
pub(crate) trait FindexMemoryTrait:
    Send
    + Sync
    + MemoryADT<Address = Address<ADDRESS_LENGTH>, Word = [u8; WORD_LENGTH], Error = MemoryError>
{
    //
    // Memory R/W ops
    //
}

#[allow(dead_code)] // code is actually used
#[async_trait]
pub(crate) trait DatabaseTraits:
    PermissionsTrait + DatasetsTrait + FindexMemoryTrait
{
}
