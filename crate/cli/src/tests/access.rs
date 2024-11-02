use crate::{
    actions::access::{GrantAccess, RevokeAccess},
    error::{result::CliResult, CliError},
    tests::{utils::recover_cmd_logs, PROG_NAME},
};
use assert_cmd::prelude::*;
use cosmian_findex_client::FINDEX_CLI_CONF_ENV;
use regex::{Regex, RegexBuilder};
use std::process::Command;
use tracing::{debug, trace};

/// Extract the `key_uid` (prefixed by a pattern) from a text
#[allow(clippy::unwrap_used)]
pub(crate) fn extract_uid<'a>(text: &'a str, pattern: &'a str) -> Option<&'a str> {
    let formatted = format!(r"^\s*{pattern}: (?P<uid>.+?)[\s\.]*?$");
    let uid_regex: Regex = RegexBuilder::new(formatted.as_str())
        .multi_line(true)
        .build()
        .unwrap();
    uid_regex
        .captures(text)
        .and_then(|cap| cap.name("uid").map(|uid| uid.as_str()))
}

pub(crate) fn create_access_cmd(cli_conf_path: &str) -> CliResult<String> {
    let mut cmd = Command::cargo_bin(PROG_NAME)?;
    let args = vec!["create".to_owned()];
    cmd.env(FINDEX_CLI_CONF_ENV, cli_conf_path);

    cmd.arg("access-rights").args(args);
    debug!("cmd: {:?}", cmd);
    let output = recover_cmd_logs(&mut cmd);
    if output.status.success() {
        let findex_output = std::str::from_utf8(&output.stdout)?;
        trace!("findex_output: {}", findex_output);
        let unique_identifier = extract_uid(findex_output, "New access successfully created")
            .ok_or_else(|| {
                CliError::Default("failed extracting the unique identifier".to_owned())
            })?;
        return Ok(unique_identifier.to_owned());
    }
    Err(CliError::Default(
        std::str::from_utf8(&output.stderr)?.to_owned(),
    ))
}

pub(crate) fn grant_access_cmd(cli_conf_path: &str, action: GrantAccess) -> CliResult<String> {
    let mut cmd = Command::cargo_bin(PROG_NAME)?;
    let args = vec![
        "grant".to_owned(),
        "--user".to_owned(),
        action.user.clone(),
        "--index-id".to_owned(),
        action.index_id,
        "--role".to_owned(),
        action.role,
    ];
    cmd.env(FINDEX_CLI_CONF_ENV, cli_conf_path);

    cmd.arg("access-rights").args(args);
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

pub(crate) fn revoke_access_cmd(cli_conf_path: &str, action: RevokeAccess) -> CliResult<String> {
    let mut cmd = Command::cargo_bin(PROG_NAME)?;
    let args = vec![
        "revoke".to_owned(),
        "--user".to_owned(),
        action.user.clone(),
        "--index-id".to_owned(),
        action.index_id,
    ];
    cmd.env(FINDEX_CLI_CONF_ENV, cli_conf_path);

    cmd.arg("access-rights").args(args);
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
