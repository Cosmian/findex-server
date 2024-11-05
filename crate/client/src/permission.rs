use std::fmt::Display;

use clap::ValueEnum;

#[derive(Clone, Debug, ValueEnum)]
pub enum Permission {
    Read = 0,
    Write = 1,
    Admin = 2,
}

impl Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Admin => "admin",
        };
        write!(f, "{s}")
    }
}
