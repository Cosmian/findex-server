use async_trait::async_trait;
use cloudproof_findex::{
    db_interfaces::{redis::FindexTable, rest::UpsertData},
    reexport::cosmian_findex::{
        TokenToEncryptedValueMap, TokenWithEncryptedValueList, Tokens, ENTRY_LENGTH, LINK_LENGTH,
    },
};

use crate::{core::Role, error::result::FResult};

#[async_trait]
pub(crate) trait Database: Sync + Send {
    async fn fetch_entries(
        &self,
        index_id: &str,
        tokens: Tokens,
    ) -> FResult<TokenWithEncryptedValueList<ENTRY_LENGTH>>;

    async fn fetch_chains(
        &self,
        index_id: &str,
        tokens: Tokens,
    ) -> FResult<TokenWithEncryptedValueList<LINK_LENGTH>>;

    async fn upsert_entries(
        &self,
        index_id: &str,
        upsert_data: UpsertData<ENTRY_LENGTH>,
    ) -> FResult<TokenToEncryptedValueMap<ENTRY_LENGTH>>;

    async fn insert_chains(
        &self,
        index_id: &str,
        items: TokenToEncryptedValueMap<LINK_LENGTH>,
    ) -> FResult<()>;

    async fn delete(
        &self,
        index_id: &str,
        findex_table: FindexTable,
        tokens: Tokens,
    ) -> FResult<()>;

    async fn dump_tokens(&self, index_id: &str) -> FResult<Tokens>;

    async fn create_access(&self, user_id: &str) -> FResult<String>;
    async fn get_access(&self, user_id: &str, index_id: &str) -> FResult<Role>;
    #[allow(dead_code)]
    async fn grant_access(&self, user_id: &str, role: Role, index_id: &str) -> FResult<String>;
    #[allow(dead_code)]
    async fn revoke_access(&self, user_id: &str, role: Role, index_id: &str) -> FResult<String>;
}
