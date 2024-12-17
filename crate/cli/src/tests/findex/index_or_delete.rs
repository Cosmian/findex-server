use std::process::Command;

use assert_cmd::prelude::*;
use cosmian_findex_client::FINDEX_CLI_CONF_ENV;
use tracing::debug;

use crate::{
    actions::findex::index_or_delete::IndexOrDeleteAction,
    error::{result::CliResult, CliError},
    tests::{utils::recover_cmd_logs, PROG_NAME},
};

#[allow(clippy::unwrap_used)]
pub(crate) fn index_or_delete_cmd(
    cli_conf_path: &str,
    command: &str,
    action: IndexOrDeleteAction,
) -> CliResult<String> {
    let mut cmd = Command::cargo_bin(PROG_NAME)?;
    let args = vec![
        "--key".to_owned(),
        String::from_utf8(action.findex_parameters.key.to_ascii_uppercase())
            .expect("Invalid UTF-8"),
        "--index-id".to_owned(),
        "--index-id".to_owned(),
        action.findex_parameters.index_id.to_string(),
        "--csv".to_owned(),
        action.csv.to_str().unwrap().to_owned(),
    ];
    cmd.env(FINDEX_CLI_CONF_ENV, cli_conf_path);

    cmd.arg(command).args(args);
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
