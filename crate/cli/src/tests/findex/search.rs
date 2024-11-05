use std::process::Command;

use assert_cmd::prelude::*;
use cosmian_rest_client::FINDEX_CLI_CONF_ENV;
use tracing::debug;

use crate::{
    actions::findex::search::SearchAction,
    error::{result::CliResult, CliError},
    tests::{utils::recover_cmd_logs, PROG_NAME},
};

pub(crate) fn search_cmd(cli_conf_path: &str, action: SearchAction) -> CliResult<String> {
    let mut args = vec![
        "--key".to_owned(),
        action.findex_parameters.key.clone(),
        "--label".to_owned(),
        action.findex_parameters.label,
        "--index-id".to_owned(),
        action.findex_parameters.index_id.to_string(),
    ];

    for word in action.keyword {
        args.push("--keyword".to_owned());
        args.push(word);
    }
    let mut cmd = Command::cargo_bin(PROG_NAME)?;
    cmd.env(FINDEX_CLI_CONF_ENV, cli_conf_path);

    cmd.arg("search").args(args);
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
