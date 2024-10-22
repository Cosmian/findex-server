use crate::{
    actions::findex::search::SearchAction,
    error::{result::CliResult, CliError},
    tests::{utils::recover_cmd_logs, PROG_NAME},
};
use assert_cmd::prelude::*;
use cosmian_findex_client::FINDEX_CLI_CONF_ENV;
use std::process::Command;
use tracing::debug;

pub(crate) fn search_cmd(cli_conf_path: &str, action: SearchAction) -> CliResult<String> {
    let mut args = vec!["search".to_owned()];

    args.push("--key".to_owned());
    args.push(action.findex_parameters.key.clone());

    args.push("--label".to_owned());
    args.push(action.findex_parameters.label);

    for word in action.word {
        args.push("--word".to_owned());
        args.push(word);
    }
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
