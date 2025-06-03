use std::{
    fmt::{self, Display},
    path::PathBuf,
};

use url::Url;

pub enum DbParams {
    Redis(Url),
    Sqlite(PathBuf),
}

impl DbParams {
    /// Return the name of the database type
    #[must_use]
    pub const fn db_name(&self) -> &str {
        match &self {
            Self::Redis(_) => "Redis",
            Self::Sqlite(_) => "SQLite",
        }
    }
}

impl Default for DbParams {
    #[allow(clippy::expect_used)] // Won't panic because the URL is valid
    fn default() -> Self {
        Self::Redis(Url::parse("redis://localhost:6379").expect("Invalid default URL"))
    }
}

impl Display for DbParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Redis(url) => {
                write!(f, "redis: {}", redact_url(url))
            }
            Self::Sqlite(path) => {
                write!(f, "sqlite: {}", path.display())
            }
        }
    }
}

/// Redact the username and password from the URL for logging purposes
#[allow(clippy::expect_used)]
fn redact_url(original: &Url) -> Url {
    let mut url = original.clone();

    if url.username() != "" {
        url.set_username("****").expect("masking username failed");
    }
    if url.password().is_some() {
        url.set_password(Some("****"))
            .expect("masking password failed");
    }
    url
}

impl std::fmt::Debug for DbParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{}", &self))
    }
}
