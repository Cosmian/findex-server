use std::path::PathBuf;

use crate::config::{ClapConfig, DBConfig, HttpConfig, JwtAuthConfig, WorkspaceConfig};

#[test]
fn test_toml() {
    let config = ClapConfig {
        db: DBConfig {
            database_type: Some("[redis-findex, postgresql,...]".to_owned()),
            database_url: Some("[redis urls]".to_owned()),
            sqlite_path: PathBuf::from("[sqlite path]"),
            redis_master_password: Some("[redis master password]".to_owned()),
            redis_findex_label: Some("[redis findex label]".to_owned()),
            clear_database: false,
        },
        http: HttpConfig {
            port: 443,
            hostname: "[hostname]".to_owned(),
            https_p12_file: Some(PathBuf::from("[https p12 file]")),
            https_p12_password: Some("[https p12 password]".to_owned()),
            authority_cert_file: Some(PathBuf::from("[authority cert file]")),
            api_token_id: None,
        },
        auth: JwtAuthConfig {
            jwt_issuer_uri: Some(vec![
                "[jwt issuer uri 1]".to_owned(),
                "[jwt issuer uri 2]".to_owned(),
            ]),
            jwks_uri: Some(vec!["[jwks uri 1]".to_owned(), "[jwks uri 2]".to_owned()]),
            jwt_audience: Some(vec![
                "[jwt audience 1]".to_owned(),
                "[jwt audience 2]".to_owned(),
            ]),
        },
        workspace: WorkspaceConfig {
            root_data_path: PathBuf::from("[root data path]"),
            tmp_path: PathBuf::from("[tmp path]"),
        },
        default_username: "[default username]".to_owned(),
        force_default_username: false,
    };

    let toml_string = r#"
default_username = "[default username]"
force_default_username = false

[db]
database_type = "[redis-findex, postgresql,...]"
database_url = "[redis urls]"
sqlite_path = "[sqlite path]"
redis_master_password = "[redis master password]"
redis_findex_label = "[redis findex label]"
clear_database = false

[http]
port = 443
hostname = "[hostname]"
https_p12_file = "[https p12 file]"
https_p12_password = "[https p12 password]"
authority_cert_file = "[authority cert file]"

[auth]
jwt_issuer_uri = ["[jwt issuer uri 1]", "[jwt issuer uri 2]"]
jwks_uri = ["[jwks uri 1]", "[jwks uri 2]"]
jwt_audience = ["[jwt audience 1]", "[jwt audience 2]"]

[workspace]
root_data_path = "[root data path]"
tmp_path = "[tmp path]"

"#;

    assert_eq!(toml_string.trim(), toml::to_string(&config).unwrap().trim());
}
