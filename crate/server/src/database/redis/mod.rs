#![allow(clippy::blocks_in_conditions)] //todo(manu): fix it

pub(crate) use findex::ServerRedis;

pub(crate) const WORD_LENGTH: usize = 129;

use super::DatabaseTraits;

mod datasets;
mod findex;
mod permissions;

impl DatabaseTraits for ServerRedis<WORD_LENGTH> {}
