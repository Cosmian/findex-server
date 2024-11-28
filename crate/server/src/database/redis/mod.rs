#![allow(clippy::blocks_in_conditions)] //todo(manu): fix it

pub(crate) use instance::Redis;

use super::DatabaseTraits;

mod datasets;
mod findex;
mod instance;
mod permissions;

impl DatabaseTraits for Redis {}
