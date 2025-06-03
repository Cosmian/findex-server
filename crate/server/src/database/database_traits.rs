use async_trait::async_trait;
use cosmian_findex::MemoryADT;
use cosmian_findex_structs::{EncryptedEntries, Permission, Permissions, Uuids};
use uuid::Uuid;

use super::findex_database::DatabaseResult;
use crate::config::DatabaseType;

#[async_trait]
pub(crate) trait PermissionsTrait: Sync + Send {
    //
    // Permissions
    //
    async fn create_index_id(&self, user_id: &str) -> DatabaseResult<Uuid>;
    async fn get_permissions(&self, user_id: &str) -> DatabaseResult<Permissions>;
    async fn get_permission(&self, user_id: &str, index_id: &Uuid) -> DatabaseResult<Permission>;
    async fn set_permission(
        &self,
        user_id: &str,
        permission: Permission,
        index_id: &Uuid,
    ) -> DatabaseResult<()>;
    async fn revoke_permission(&self, user_id: &str, index_id: &Uuid) -> DatabaseResult<()>;
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
    ) -> DatabaseResult<()>;
    async fn dataset_delete_entries(&self, index_id: &Uuid, uuids: &Uuids) -> DatabaseResult<()>;
    async fn dataset_get_entries(
        &self,
        index_id: &Uuid,
        uuids: &Uuids,
    ) -> DatabaseResult<EncryptedEntries>;
}

#[async_trait]
pub(crate) trait InstantiationTrait: Sync + Send {
    // The trait `InstantiationTrait` is a constructor trait, which means that
    // we can not call the `instantiate` method with `&self` hence the need for this
    // enum to know which type of database we are instantiating.
    async fn instantiate(
        db_type: DatabaseType,
        db_url: &str,
        clear_database: bool,
    ) -> DatabaseResult<Self>
    where
        Self: Sized;
}
#[allow(dead_code)] // false positive, used in crate/server/src/database/redis/mod.rs
#[async_trait]
pub(crate) trait DatabaseTraits:
    PermissionsTrait + DatasetsTrait + InstantiationTrait + MemoryADT
{
}
