use crate::database::database_traits::FindexMemoryTrait;

use super::{Redis, WORD_LENGTH};

impl FindexMemoryTrait for Redis<WORD_LENGTH> {}
