use async_trait::async_trait;
use cloudproof_findex::{
    db_interfaces::{redis::FindexTable, rest::UpsertData},
    reexport::cosmian_findex::{
        TokenToEncryptedValueMap, TokenWithEncryptedValueList, Tokens, ENTRY_LENGTH, LINK_LENGTH,
    },
};

use crate::error::result::FResult;

#[async_trait]
pub(crate) trait Database: Sync + Send {
    async fn fetch_entries(
        &self,
        tokens: Tokens,
    ) -> FResult<TokenWithEncryptedValueList<ENTRY_LENGTH>>;

    async fn fetch_chains(
        &self,
        tokens: Tokens,
    ) -> FResult<TokenWithEncryptedValueList<LINK_LENGTH>>;

    async fn upsert_entries(
        &self,
        upsert_data: UpsertData<ENTRY_LENGTH>,
    ) -> FResult<TokenToEncryptedValueMap<ENTRY_LENGTH>>;

    async fn insert_chains(&self, items: TokenToEncryptedValueMap<LINK_LENGTH>) -> FResult<()>;

    async fn delete(&self, findex_table: FindexTable, tokens: Tokens) -> FResult<()>;

    async fn dump_tokens(&self) -> FResult<Tokens>;
}
