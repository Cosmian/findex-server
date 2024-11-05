use async_trait::async_trait;
use cloudproof_findex::{
    db_interfaces::{redis::FindexTable, rest::UpsertData},
    reexport::cosmian_findex::{
        TokenToEncryptedValueMap, TokenWithEncryptedValueList, Tokens, ENTRY_LENGTH, LINK_LENGTH,
    },
};
use uuid::Uuid;

use crate::{
    core::{Permission, Permissions},
    error::result::FResult,
};

#[async_trait]
pub(crate) trait Database: Sync + Send {
    async fn fetch_entries(
        &self,
        index_id: &Uuid,
        tokens: Tokens,
    ) -> FResult<TokenWithEncryptedValueList<ENTRY_LENGTH>>;

    async fn fetch_chains(
        &self,
        index_id: &Uuid,
        tokens: Tokens,
    ) -> FResult<TokenWithEncryptedValueList<LINK_LENGTH>>;

    async fn upsert_entries(
        &self,
        index_id: &Uuid,
        upsert_data: UpsertData<ENTRY_LENGTH>,
    ) -> FResult<TokenToEncryptedValueMap<ENTRY_LENGTH>>;

    async fn insert_chains(
        &self,
        index_id: &Uuid,
        items: TokenToEncryptedValueMap<LINK_LENGTH>,
    ) -> FResult<()>;

    async fn delete(
        &self,
        index_id: &Uuid,
        findex_table: FindexTable,
        tokens: Tokens,
    ) -> FResult<()>;

    async fn dump_tokens(&self, index_id: &Uuid) -> FResult<Tokens>;

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
