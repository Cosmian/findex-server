use std::str::FromStr;

use crate::{
    error::{result::FResult, server::FindexServerError},
    findex_server_bail,
};

#[repr(u8)]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Role {
    Read = 0,
    Write = 1,
    Admin = 2,
}

#[allow(clippy::as_conversions)]
impl From<Role> for u8 {
    fn from(table: Role) -> Self {
        table as Self
    }
}

impl FromStr for Role {
    type Err = FindexServerError;

    fn from_str(s: &str) -> FResult<Self> {
        match s {
            "read" => Ok(Self::Read),
            "write" => Ok(Self::Write),
            "admin" => Ok(Self::Admin),
            _ => findex_server_bail!("Invalid role: {}", s),
        }
    }
}
