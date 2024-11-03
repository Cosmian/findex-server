use std::{fmt::Display, str::FromStr};

use crate::{
    error::{result::FResult, server::FindexServerError},
    findex_server_bail,
};

#[repr(u8)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
pub(crate) enum Permission {
    Read = 0,
    Write = 1,
    Admin = 2,
}

#[allow(clippy::as_conversions)]
impl From<Permission> for u8 {
    fn from(table: Permission) -> Self {
        table as Self
    }
}

impl TryFrom<u8> for Permission {
    type Error = FindexServerError;

    fn try_from(value: u8) -> FResult<Self> {
        match value {
            0 => Ok(Self::Read),
            1 => Ok(Self::Write),
            2 => Ok(Self::Admin),
            _ => findex_server_bail!("Invalid permission: {}", value),
        }
    }
}

impl FromStr for Permission {
    type Err = FindexServerError;

    fn from_str(s: &str) -> FResult<Self> {
        match s {
            "reader" => Ok(Self::Read),
            "writer" => Ok(Self::Write),
            "admin" => Ok(Self::Admin),
            _ => findex_server_bail!("Invalid permission: {}", s),
        }
    }
}

impl Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Read => "reader",
            Self::Write => "writer",
            Self::Admin => "admin",
        };
        write!(f, "{s}")
    }
}
