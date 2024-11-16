#![allow(clippy::blocks_in_conditions)] //todo(manu): fix it

pub(crate) use findex::Redis;

use super::DatabaseTraits;

mod datasets;
mod findex;
mod permissions;

impl DatabaseTraits for Redis {}
