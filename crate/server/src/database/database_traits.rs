use async_trait::async_trait;
use cloudproof_findex::{
    db_interfaces::{redis::FindexTable, rest::UpsertData},
    reexport::cosmian_findex::{
        TokenToEncryptedValueMap, TokenWithEncryptedValueList, Tokens, ENTRY_LENGTH, LINK_LENGTH,
    },
};
use cosmian_findex_structs::{EncryptedEntries, Permission, Permissions, Uuids};
use uuid::Uuid;

use crate::error::result::FResult;

#[async_trait]
pub(crate) trait FindexTrait: Sync + Send {
    //
    // Findex v6
    //
    async fn findex_fetch_entries(
        &self,
        index_id: &Uuid,
        tokens: Tokens,
    ) -> FResult<TokenWithEncryptedValueList<ENTRY_LENGTH>>;
    async fn findex_fetch_chains(
        &self,
        index_id: &Uuid,
        tokens: Tokens,
    ) -> FResult<TokenWithEncryptedValueList<LINK_LENGTH>>;
    async fn findex_upsert_entries(
        &self,
        index_id: &Uuid,
        upsert_data: UpsertData<ENTRY_LENGTH>,
    ) -> FResult<TokenToEncryptedValueMap<ENTRY_LENGTH>>;
    async fn findex_insert_chains(
        &self,
        index_id: &Uuid,
        items: TokenToEncryptedValueMap<LINK_LENGTH>,
    ) -> FResult<()>;
    async fn findex_delete(
        &self,
        index_id: &Uuid,
        findex_table: FindexTable,
        tokens: Tokens,
    ) -> FResult<()>;
    async fn findex_dump_tokens(&self, index_id: &Uuid) -> FResult<Tokens>;
}

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

#[async_trait]
pub(crate) trait DatabaseTraits: FindexTrait + PermissionsTrait + DatasetsTrait {}
