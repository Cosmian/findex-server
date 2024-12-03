use std::fmt::{self};

use clap::Parser;
use serde::{Deserialize, Serialize};

use super::{DBConfig, HttpConfig, JwtAuthConfig};

const DEFAULT_USERNAME: &str = "admin";

impl Default for ClapConfig {
    fn default() -> Self {
        Self {
            db: DBConfig::default(),
            http: HttpConfig::default(),
            auth: JwtAuthConfig::default(),
            default_username: DEFAULT_USERNAME.to_owned(),
            force_default_username: false,
        }
    }
}

#[derive(Parser, Serialize, Deserialize, PartialEq, Eq)]
#[clap(version, about, long_about = None)]
#[serde(default)]
pub struct ClapConfig {
    #[clap(flatten)]
    pub db: DBConfig,

    #[clap(flatten)]
    pub http: HttpConfig,

    #[clap(flatten)]
    pub auth: JwtAuthConfig,

    /// The default username to use when no authentication method is provided
    #[clap(long, env = "FINDEX_SERVER_DEFAULT_USERNAME", default_value = DEFAULT_USERNAME)]
    pub default_username: String,

    /// When an authentication method is provided, perform the authentication
    /// but always use the default username instead of the one provided by the
    /// authentication method
    #[clap(long, env = "FINDEX_SERVER_FORCE_DEFAULT_USERNAME")]
    pub force_default_username: bool,
}

impl fmt::Debug for ClapConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut x = f.debug_struct("");
        let x = x.field("db", &self.db);
        let x = if self.auth.jwt_issuer_uri.is_some() {
            x.field("auth", &self.auth)
        } else {
            x
        };
        let x = x.field("Findex server http", &self.http);
        let x = x.field("default username", &self.default_username);
        let x = x.field("force default username", &self.force_default_username);
        x.finish()
    }
}
