use async_trait::async_trait;

use async_trait::async_trait;
use cloudproof_findex::{
    db_interfaces::{
        redis::{FindexTable, TABLE_PREFIX_LENGTH},
        rest::UpsertData,
    },
    reexport::cosmian_findex::{
        CoreError, EncryptedValue, Token, TokenToEncryptedValueMap, TokenWithEncryptedValueList,
        Tokens, ENTRY_LENGTH, LINK_LENGTH,
    },
};
use redis::{pipe, AsyncCommands};
use tracing::{instrument, trace};
use uuid::Uuid;

use super::{Redis, WORD_LENGTH};

#[async_trait]
impl FindexMemoryTrait for Redis<WORD_LENGTH> {}
