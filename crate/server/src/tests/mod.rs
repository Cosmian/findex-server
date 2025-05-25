#[allow(clippy::indexing_slicing)] // this is a test. Also the indexing is correctly done and won't panic
#[cfg(test)]
mod tests_inner {
    use crate::config::{ClapConfig, DBConfig, DatabaseType, HttpConfig, JwtAuthConfig};
    use std::{io::Write, path::PathBuf};
    use tempfile::TempDir;

    fn get_database_configurations() -> (Vec<String>, Vec<DBConfig>) {
        assert_eq!(
            DatabaseType::VARIANT_COUNT,
            2,
            "If this panic occurs, it means that a new database type has been added. Please update the test to include the new database type."
        );

        let db_names = vec!["Redis".to_owned(), "Sqlite".to_owned()];

        let db_configs = vec![
            DBConfig {
                database_type: DatabaseType::Redis,
                database_url: "[some urls]".to_owned(),
                clear_database: false,
            },
            DBConfig {
                database_type: DatabaseType::Sqlite,
                database_url: "[some urls]".to_owned(),
                clear_database: false,
            },
        ];

        (db_names, db_configs)
    }

    #[test]
    fn test_toml() {
        let (db_names, db_configs) = get_database_configurations();

        for (i, db_config) in db_configs.iter().enumerate() {
            let config = ClapConfig {
                db: db_config.clone(),
                http: HttpConfig {
                    port: 443,
                    hostname: "[hostname]".to_owned(),
                    https_p12_file: Some(PathBuf::from("[https p12 file]")),
                    https_p12_password: Some("[https p12 password]".to_owned()),
                    authority_cert_file: Some(PathBuf::from("[authority cert file]")),
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
                default_username: "[default username]".to_owned(),
                force_default_username: false,
            };
            let expected_toml = format!(
                r#"
default_username = "[default username]"
force_default_username = false

[db]
database_type = "{}"
database_url = "[some urls]"
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
"#,
                db_names[i]
            );
            assert_eq!(
                expected_toml.trim(),
                toml::to_string(&config).unwrap().trim()
            );
        }
    }

    #[test]
    fn test_read_write_toml() {
        let (_, db_configs) = get_database_configurations();

        for db_config in db_configs {
            let config = ClapConfig {
                db: db_config,
                http: HttpConfig {
                    port: 443,
                    hostname: "[hostname]".to_owned(),
                    https_p12_file: Some(PathBuf::from("[https p12 file]")),
                    https_p12_password: Some("[https p12 password]".to_owned()),
                    authority_cert_file: Some(PathBuf::from("[authority cert file]")),
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
                default_username: "[default username]".to_owned(),
                force_default_username: false,
            };

            // create a temp dir
            let tmp_dir = TempDir::new().unwrap();
            let tmp_path = tmp_dir.path();
            let tmp_path = tmp_path.join("config.toml");

            let toml_string = toml::to_string(&config).unwrap();
            let mut file = std::fs::File::create(&tmp_path).unwrap();
            file.write_all(toml_string.as_bytes()).unwrap();

            let loaded_conf = std::fs::read_to_string(&tmp_path).unwrap();
            let read_config: ClapConfig = toml::from_str(&loaded_conf).unwrap();

            assert_eq!(config, read_config);
        }
    }
}
