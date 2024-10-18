pub use cosmian_findex_server::config::{DBConfig, DatabaseType, DEFAULT_SQLITE_PATH};
pub use test_server::{
    start_default_test_findex_server, start_default_test_findex_server_with_cert_auth,
    start_test_server_with_options, AuthenticationOptions, TestsContext,
};

mod test_server;

mod test_jwt;
