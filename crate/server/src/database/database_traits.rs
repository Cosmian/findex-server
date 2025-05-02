use async_trait::async_trait;
use cosmian_findex::MemoryADT;
use cosmian_findex_structs::{EncryptedEntries, Permission, Permissions, Uuids};
use uuid::Uuid;

use super::findex_database::FDBResult;

#[async_trait]
pub(crate) trait PermissionsTrait: Sync + Send {
    //
    // Permissions
    //
    async fn create_index_id(&self, user_id: &str) -> FDBResult<Uuid>;
    async fn get_permissions(&self, user_id: &str) -> FDBResult<Permissions>;
    async fn get_permission(&self, user_id: &str, index_id: &Uuid) -> FDBResult<Permission>;
    async fn set_permission(
        &self,
        user_id: &str,
        permission: Permission,
        index_id: &Uuid,
    ) -> FDBResult<()>;
    async fn revoke_permission(&self, user_id: &str, index_id: &Uuid) -> FDBResult<()>;
}

#[async_trait]
pub(crate) trait DatasetsTrait: Sync + Send {
    //
    // Dataset management
    //
    async fn dataset_add_entries(
        &self,
        index_id: &Uuid,
        entries: &EncryptedEntries,
    ) -> FDBResult<()>;
    async fn dataset_delete_entries(&self, index_id: &Uuid, uuids: &Uuids) -> FDBResult<()>;
    async fn dataset_get_entries(
        &self,
        index_id: &Uuid,
        uuids: &Uuids,
    ) -> FDBResult<EncryptedEntries>;
}

#[async_trait]
pub(crate) trait InstantializationTrait: Sync + Send {
    /// Creates a new Redis database connection instance
    async fn instantiate(db_url: &str, clear_database: bool) -> FDBResult<Self>
    where
        Self: Sized;
}
#[allow(dead_code)] // false positive, used in crate/server/src/database/redis/mod.rs
#[async_trait]
pub(crate) trait DatabaseTraits:
    PermissionsTrait + DatasetsTrait + InstantializationTrait + MemoryADT
{
}
