use crate::error::result::FResult;
use async_trait::async_trait;
use cosmian_findex::MemoryADT;
use cosmian_findex_structs::{EncryptedEntries, Permission, Permissions, Uuids};
use uuid::Uuid;

#[async_trait]
pub(crate) trait PermissionsTrait: Sync + Send {
    //
    // Permissions
    //
    async fn create_index_id(&self, user_id: &str) -> FResult<Uuid>;
    async fn get_permissions(&self, user_id: &str) -> FResult<Permissions>;
    async fn get_permission(&self, user_id: &str, index_id: &Uuid) -> FResult<Permission>;
    async fn set_permission(
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

#[allow(dead_code)] // false positive, used in crate/server/src/database/redis/mod.rs
#[async_trait]
pub(crate) trait DatabaseTraits: MemoryADT + PermissionsTrait + DatasetsTrait {}
