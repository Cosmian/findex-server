use std::process::Command;

use assert_cmd::prelude::*;
use cosmian_findex_client::FINDEX_CLI_CONF_ENV;
use cosmian_logger::log_utils::log_init;
use test_findex_server::start_default_test_findex_server;
use tracing::debug;

use crate::{
    actions::findex::{index::IndexAction, FindexParameters},
    error::{result::CliResult, CliError},
    tests::{utils::recover_cmd_logs, PROG_NAME},
};

fn findex(cli_conf_path: &str, action: IndexAction) -> CliResult<String> {
    let mut args = vec!["index".to_owned()];

    args.push("--key".to_owned());
    args.push(action.findex_parameters.key.clone());

    args.push("--label".to_owned());
    args.push(action.findex_parameters.label);

    let mut cmd = Command::cargo_bin(PROG_NAME)?;
    cmd.env(FINDEX_CLI_CONF_ENV, cli_conf_path);

    cmd.arg("findex").args(args);
    debug!("cmd: {:?}", cmd);
    let output = recover_cmd_logs(&mut cmd);
    if output.status.success() {
        let findex_output = std::str::from_utf8(&output.stdout)?;
        return Ok(findex_output.to_owned());
    }
    Err(CliError::Default(
        std::str::from_utf8(&output.stderr)?.to_owned(),
    ))
}

#[tokio::test]
#[allow(clippy::needless_return)]
pub(crate) async fn test_findex() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server().await;

    let cmd = findex(
        &ctx.owner_client_conf_path,
        IndexAction {
            findex_parameters: FindexParameters {
                key: "11223344556677889900AABBCCDDEEFF".to_owned(),
                label: "My Findex label".to_owned(),
            },
        },
    )?;

    debug!("cmd: {}", cmd);

    Ok(())
}
