mod datasets;
mod findex;
mod instance;
mod permissions;

use cosmian_findex_structs::CUSTOM_WORD_LENGTH;
pub(crate) use instance::Redis;

use crate::database::database_traits::DatabaseTraits;
impl DatabaseTraits for Redis<CUSTOM_WORD_LENGTH> {}
