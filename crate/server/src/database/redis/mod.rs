mod datasets;
mod findex;
mod instance;
mod permissions;

pub(crate) use instance::Redis;

use crate::database::database_traits::DatabaseTraits;
impl DatabaseTraits for Redis<WORD_LENGTH> {}

pub use cosmian_findex_config::{decode_fn, encode_fn, WORD_LENGTH}; // todo(hatem): remùove this line and correct bugs if necessary
