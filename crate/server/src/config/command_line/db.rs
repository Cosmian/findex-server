use std::fmt::Display;

use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{config::params::DbParams, error::result::FResult, findex_server_error};

#[derive(ValueEnum, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum DatabaseType {
    Redis,
}

/// Configuration for the database
#[derive(Args, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct DBConfig {
    /// The database type of the Findex server
    /// - redis: Redis database. The Redis url must be provided
    #[clap(
        long,
        env("FINDEX_SERVER_DATABASE_TYPE"),
        default_value = "redis",
        verbatim_doc_comment
    )]
    pub database_type: DatabaseType,

    /// The url of the database
    #[clap(
        long,
        env = "FINDEX_SERVER_DATABASE_URL",
        required_if_eq_any([("database_type", "redis")]),
        default_value = "redis://localhost:6379"
    )]
    pub database_url: String,

    /// Clear the database on start.
    /// WARNING: This will delete ALL the data in the database
    #[clap(long, env = "FINDEX_SERVER_CLEAR_DATABASE", verbatim_doc_comment)]
    pub clear_database: bool,
}

impl Default for DBConfig {
    fn default() -> Self {
        Self {
            database_type: DatabaseType::Redis,
            database_url: "redis://localhost:6379".to_owned(),
            clear_database: false,
        }
    }
}

impl Display for DBConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.database_type {
            DatabaseType::Redis => write!(f, "redis: {}", self.database_url),
        }?;
        write!(f, ", clear_database?: {}", self.clear_database)
    }
}

impl std::fmt::Debug for DBConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", &self))
    }
}

impl DBConfig {
    /// Initialize the DB parameters based on the command-line parameters
    ///
    /// # Parameters
    /// - `workspace`: The workspace configuration used to determine the public
    ///   and shared paths
    ///
    /// # Returns
    /// - The DB parameters
    pub(crate) fn init(&self) -> FResult<DbParams> {
        match self.database_type {
            DatabaseType::Redis => {
                let url = ensure_url(self.database_url.as_str(), "FINDEX_SERVER_REDIS_URL")?;
                Ok(DbParams::Redis(url))
            }
        }
    }
}

fn ensure_url(database_url: &str, alternate_env_variable: &str) -> FResult<Url> {
    let url = if database_url.is_empty() {
        std::env::var(alternate_env_variable).map_err(|_e| {
            findex_server_error!(
                "No database URL supplied either using the 'database-url' option, or the \
                 FINDEX_SERVER_DATABASE_URL or the {alternate_env_variable} environment variables",
            )
        })?
    } else {
        database_url.to_owned()
    };
    let url = Url::parse(&url)?;
    Ok(url)
}
