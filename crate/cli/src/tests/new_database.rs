use std::process::Command;

use assert_cmd::prelude::*;
use cosmian_findex_client::FINDEX_CLI_CONF_ENV;
use cosmian_logger::log_utils::log_init;
use predicates::prelude::*;
use test_findex_server::start_default_test_findex_server;
use tracing::info;

use crate::{
    error::result::CliResult,
    tests::{utils::recover_cmd_logs, PROG_NAME},
};

#[tokio::test]
#[allow(clippy::needless_return)]
pub(crate) async fn test_new_database() -> CliResult<()> {
    log_init(option_env!("RUST_LOG"));
    let ctx = start_default_test_findex_server().await;

    if ctx.owner_client_conf.findex_database_secret.is_none() {
        info!("Skipping test_new_database as backend not sqlite-enc");
        return Ok(());
    }

    let mut cmd = Command::cargo_bin(PROG_NAME)?;
    cmd.env(FINDEX_CLI_CONF_ENV, &ctx.owner_client_conf_path);

    cmd.arg("new-database");
    recover_cmd_logs(&mut cmd);
    cmd.assert().success().stdout(predicate::str::contains(
        "A new user encrypted database is configured",
    ));

    Ok(())
}

#[tokio::test]
#[allow(clippy::needless_return, clippy::panic_in_result_fn)]
pub(crate) async fn test_conf_does_not_exist() -> CliResult<()> {
    log_init(option_env!("RUST_LOG"));
    let ctx = start_default_test_findex_server().await;

    if ctx.owner_client_conf.findex_database_secret.is_none() {
        info!("Skipping test_conf_does_not_exist as backend not sqlite-enc");
        return Ok(());
    }

    let mut cmd = Command::cargo_bin(PROG_NAME)?;
    cmd.env(
        FINDEX_CLI_CONF_ENV,
        "test_data/configs/kms_bad_group_id.bad",
    );

    cmd.arg("ec").args(vec!["keys", "create"]);
    let output = recover_cmd_logs(&mut cmd);
    assert!(!output.status.success());
    Ok(())
}
