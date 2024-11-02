use std::{fmt::Display, str::FromStr};

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

impl TryFrom<u8> for Role {
    type Error = FindexServerError;

    fn try_from(value: u8) -> FResult<Self> {
        match value {
            0 => Ok(Self::Read),
            1 => Ok(Self::Write),
            2 => Ok(Self::Admin),
            _ => findex_server_bail!("Invalid role: {}", value),
        }
    }
}

impl FromStr for Role {
    type Err = FindexServerError;

    fn from_str(s: &str) -> FResult<Self> {
        match s {
            "reader" => Ok(Self::Read),
            "writer" => Ok(Self::Write),
            "admin" => Ok(Self::Admin),
            _ => findex_server_bail!("Invalid role: {}", s),
        }
    }
}

impl Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Read => "reader",
            Self::Write => "writer",
            Self::Admin => "admin",
        };
        write!(f, "{s}")
    }
}
