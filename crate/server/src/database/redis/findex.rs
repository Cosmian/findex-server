use async_trait::async_trait;
use cosmian_findex_structs::WORD_LENGTH;

use crate::database::database_traits::FindexMemoryTrait;

use super::Redis;

#[async_trait]
impl FindexMemoryTrait for Redis<WORD_LENGTH> {}
