use async_trait::async_trait;

use crate::database::database_traits::FindexMemoryTrait;

use super::{Redis, WORD_LENGTH};

#[async_trait]
impl FindexMemoryTrait for Redis<WORD_LENGTH> {}
