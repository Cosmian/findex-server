use std::process::Command;

use assert_cmd::prelude::*;
use cosmian_config_utils::ConfigUtils;
use cosmian_findex_client::{FindexClientConfig, FindexRestClient, FINDEX_CLI_CONF_ENV};
use tracing::debug;
use uuid::Uuid;

use crate::{
    actions::permissions::{CreateIndex, GrantPermission, ListPermissions, RevokePermission},
    error::{result::CliResult, CliError},
    tests::{utils::recover_cmd_logs, PROG_NAME},
};

pub(crate) async fn create_index_id_cmd(cli_conf_path: &str) -> CliResult<Uuid> {
    let findex_rest_client = FindexRestClient::new(FindexClientConfig::from_toml(cli_conf_path)?)?;
    CreateIndex.run(&findex_rest_client).await
}

pub(crate) fn list_permission_cmd(
    cli_conf_path: &str,
    action: &ListPermissions,
) -> CliResult<String> {
    let mut cmd = Command::cargo_bin(PROG_NAME)?;
    let args = vec!["list".to_owned(), "--user".to_owned(), action.user.clone()];
    cmd.env(FINDEX_CLI_CONF_ENV, cli_conf_path);

    cmd.arg("permissions").args(args);
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

pub(crate) fn grant_permission_cmd(
    cli_conf_path: &str,
    action: &GrantPermission,
) -> CliResult<String> {
    let mut cmd = Command::cargo_bin(PROG_NAME)?;
    let args = vec![
        "grant".to_owned(),
        "--user".to_owned(),
        action.user.clone(),
        "--index-id".to_owned(),
        action.index_id.to_string(),
        "--permission".to_owned(),
        action.permission.to_string(),
    ];
    cmd.env(FINDEX_CLI_CONF_ENV, cli_conf_path);

    cmd.arg("permissions").args(args);
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

pub(crate) fn revoke_permission_cmd(
    cli_conf_path: &str,
    action: &RevokePermission,
) -> CliResult<String> {
    let mut cmd = Command::cargo_bin(PROG_NAME)?;
    let args = vec![
        "revoke".to_owned(),
        "--user".to_owned(),
        action.user.clone(),
        "--index-id".to_owned(),
        action.index_id.to_string(),
    ];
    cmd.env(FINDEX_CLI_CONF_ENV, cli_conf_path);

    cmd.arg("permissions").args(args);
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
