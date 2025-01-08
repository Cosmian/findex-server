use std::{collections::HashMap, ops::Deref, process::Command};

use assert_cmd::prelude::*;
use base64::{Engine, engine::general_purpose};
use cosmian_findex_client::FINDEX_CLI_CONF_ENV;
use cosmian_findex_structs::EncryptedEntries;
use cosmian_logger::log_init;
use test_findex_server::start_default_test_findex_server;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::{
    actions::datasets::{AddEntries, DeleteEntries, GetEntries},
    error::{CliError, result::CliResult},
    tests::{PROG_NAME, utils::recover_cmd_logs},
};

pub(crate) fn dataset_add_entries_cmd(
    cli_conf_path: &str,
    action: &AddEntries,
) -> CliResult<String> {
    let mut cmd = Command::cargo_bin(PROG_NAME)?;
    let mut args = vec![
        "add".to_owned(),
        "--index-id".to_owned(),
        action.index_id.to_string(),
    ];
    for (entry_id, data) in &action.entries {
        args.push("-D".to_owned());
        args.push(format!("{entry_id}={data}"));
    }
    cmd.env(FINDEX_CLI_CONF_ENV, cli_conf_path);

    cmd.arg("datasets").args(args);
    let output = recover_cmd_logs(&mut cmd);
    if output.status.success() {
        let findex_output = std::str::from_utf8(&output.stdout)?;
        return Ok(findex_output.to_owned());
    }
    Err(CliError::Default(
        std::str::from_utf8(&output.stderr)?.to_owned(),
    ))
}

pub(crate) fn dataset_delete_entries_cmd(
    cli_conf_path: &str,
    delete_entries: &DeleteEntries,
) -> CliResult<String> {
    let mut cmd = Command::cargo_bin(PROG_NAME)?;
    let mut args = vec![
        "delete".to_owned(),
        "--index-id".to_owned(),
        delete_entries.index_id.to_string(),
    ];
    for uuid in delete_entries.uuids.clone() {
        args.push("--uuids".to_owned());
        args.push(uuid.to_string());
    }
    cmd.env(FINDEX_CLI_CONF_ENV, cli_conf_path);

    cmd.arg("datasets").args(args);
    let output = recover_cmd_logs(&mut cmd);
    if output.status.success() {
        let findex_output = std::str::from_utf8(&output.stdout)?;
        return Ok(findex_output.to_owned());
    }
    Err(CliError::Default(
        std::str::from_utf8(&output.stderr)?.to_owned(),
    ))
}

pub(crate) fn datasets_get_entries_cmd(
    cli_conf_path: &str,
    get_entries: &GetEntries,
) -> CliResult<EncryptedEntries> {
    let mut cmd = Command::cargo_bin(PROG_NAME)?;
    let mut args = vec![
        "get".to_owned(),
        "--index-id".to_owned(),
        get_entries.index_id.to_string(),
    ];
    for uuid in get_entries.uuids.clone() {
        args.push("--uuids".to_owned());
        args.push(uuid.to_string());
    }
    cmd.env(FINDEX_CLI_CONF_ENV, cli_conf_path);

    cmd.arg("datasets").args(args);
    let output = recover_cmd_logs(&mut cmd);
    if output.status.success() {
        let findex_output = std::str::from_utf8(&output.stdout)?
            .split("index_id")
            .next()
            .unwrap_or("");

        return parse_entries(findex_output);
    }
    Err(CliError::Default(
        std::str::from_utf8(&output.stderr)?.to_owned(),
    ))
}

#[allow(clippy::indexing_slicing)]
fn parse_entries(s: &str) -> CliResult<EncryptedEntries> {
    let mut entries_map = HashMap::new();
    let mut i = 1;
    for line in s.lines() {
        // first line is faulty on RHL9
        let safe_line = if line.contains("return=") {
            line.split(", Entry Value: ").collect::<Vec<&str>>()[1]
        } else {
            line
        };
        warn!("safe_line {i} ****: {safe_line}");
        i += 1;
        let parts: Vec<&str> = safe_line.split(", Entry Value: ").collect();
        if parts.len() == 2 {
            let index_id = parts[0].replace("Entry ID: ", "");
            let entry = parts[1].to_owned();
            let uid = Uuid::parse_str(&index_id).map_err(|e| {
                CliError::Default(format!(
                    "Error parsing Uuid: {e}. $$$$$$$$$$$$. Original index_id : {index_id}"
                ))
            });
            if uid.is_err() {
                warn!("Error parsing Uuid: $$$$$$$$$$$$. Original index_id : {index_id}");
                continue;
            };
            #[allow(clippy::unwrap_used)]
            entries_map.insert(uid.unwrap(), general_purpose::STANDARD.decode(entry)?);
        } else {
            warn!("Invalid line: $$$$$$$$ {line}");
        }
    }
    Ok(EncryptedEntries::from(entries_map))
}

#[tokio::test]
#[allow(clippy::panic_in_result_fn, clippy::print_stdout)]
pub(crate) async fn test_datasets() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server().await;

    let index_id = Uuid::new_v4();

    // Dataset entries IDs
    let entries_number = 100;
    let encrypted_entries: Vec<(Uuid, String)> = (1..=entries_number)
        .map(|i| {
            let entry_id = Uuid::new_v4();
            let data = general_purpose::STANDARD.encode(format!("entry{i}"));
            (entry_id, data)
        })
        .collect();

    let uuids: Vec<Uuid> = encrypted_entries.iter().map(|(uuid, _)| *uuid).collect();
    debug!("uuids: {:?}", uuids);

    // Add entries to the dataset
    dataset_add_entries_cmd(&ctx.owner_client_conf_path, &AddEntries {
        index_id,
        entries: encrypted_entries,
    })?;

    // Get the added entries from the dataset
    let added_entries = datasets_get_entries_cmd(&ctx.owner_client_conf_path, &GetEntries {
        index_id,
        uuids: uuids.clone(),
    })?;
    assert_eq!(added_entries.len(), entries_number);

    dataset_delete_entries_cmd(&ctx.owner_client_conf_path, &DeleteEntries {
        index_id,
        uuids: added_entries.get_uuids().deref().to_owned(),
    })?;

    // Get the added entries from the dataset
    let deleted_entries =
        datasets_get_entries_cmd(&ctx.owner_client_conf_path, &GetEntries { index_id, uuids })?;
    // println!("added_entries: {added_entries}");
    assert_eq!(deleted_entries.len(), 0);

    Ok(())
}
