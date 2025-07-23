use cosmian_logger::log_init;
use test_findex_server::{AuthenticationOptions, get_db_config, start_test_server_with_options};
use tracing::{info, trace};

use crate::error::result::FindexCliResult;

// let us not make other test cases fail
const PORT: u16 = 6667;

//TODO: please check equivalent - but more detailed - tests in src/tests/kms/auth_tests.rs

#[tokio::test]
pub(crate) async fn test_all_authentications() -> FindexCliResult<()> {
    log_init(None);
    let default_db_config = get_db_config();
    trace!(
        "TESTS: using db {:?} on {:?}",
        default_db_config.database_type, default_db_config.database_url
    );
    // SCENARIO 1: plaintext no auth
    info!("Testing server with no auth");
    let options = AuthenticationOptions {
        use_jwt_token: false,
        use_https: false,
        use_client_cert: false,
        use_api_token: false,
        ..Default::default()
    };

    let ctx = start_test_server_with_options(default_db_config.clone(), PORT, options).await?;
    ctx.stop_server().await?;

    // SCENARIO 2: plaintext JWT token auth - successful auth with token
    info!("Testing server with JWT token auth - successful");
    let options = AuthenticationOptions {
        use_jwt_token: true,
        use_https: false,
        use_client_cert: false,
        use_api_token: false,
        ..Default::default()
    };
    // Default behavior sends valid JWT token

    let ctx = start_test_server_with_options(default_db_config.clone(), PORT, options).await?;
    ctx.stop_server().await?;

    // SCENARIO 3: tls token auth
    info!("Testing server with TLS token auth");
    let options = AuthenticationOptions {
        use_jwt_token: true,
        use_https: true,
        use_client_cert: false,
        use_api_token: false,
        ..Default::default()
    };
    // Default behavior sends valid JWT token

    let ctx = start_test_server_with_options(default_db_config.clone(), PORT, options).await?;
    ctx.stop_server().await?;

    // SCENARIO 4: Client Certificates and JWT authentication are enabled, but the user only presents a JWT token.
    info!("Testing server with both Client Certificates and JWT auth - JWT token only");
    let options = AuthenticationOptions {
        use_jwt_token: true,
        use_https: true,
        use_client_cert: true,
        use_api_token: false,
        do_not_send_client_certificate: true, // Don't send the client certificate
        ..Default::default()
    };

    let ctx = start_test_server_with_options(default_db_config.clone(), PORT, options).await?;
    ctx.stop_server().await?;

    // SCENARIO 5: Both Client Certificates and API token authentication are enabled, the user presents an API token only
    info!("Testing server with both Client Certificates and API token auth - API token only");
    let options = AuthenticationOptions {
        use_jwt_token: false,
        use_https: true,
        use_client_cert: true,
        use_api_token: true,
        do_not_send_client_certificate: true, // Don't send client certificate
        ..Default::default()
    };
    // Default behavior sends a valid API token

    let ctx = start_test_server_with_options(default_db_config.clone(), PORT, options).await?;
    ctx.stop_server().await?;

    // SCENARIO 6: Both JWT and API token authentication are enabled, user presents an API token only
    info!("Testing server with both JWT and API token auth - API token only");
    let options = AuthenticationOptions {
        use_jwt_token: true,
        use_https: false,
        use_client_cert: false,
        use_api_token: true,
        do_not_send_jwt_token: true, // Send invalid JWT token
        ..Default::default()
    };
    // Default behavior sends valid API token

    let ctx = start_test_server_with_options(default_db_config.clone(), PORT, options).await?;
    ctx.stop_server().await?;

    Ok(())
}
