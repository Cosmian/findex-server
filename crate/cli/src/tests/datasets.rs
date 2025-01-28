use crate::{
    actions::datasets::{AddEntries, DeleteEntries, GetEntries},
    error::result::CliResult,
};
use base64::{engine::general_purpose, Engine};
use cosmian_findex_client::{FindexClientConfig, FindexRestClient};
use cosmian_findex_structs::EncryptedEntries;
use cosmian_logger::log_init;
use std::{ops::Deref, path::PathBuf};
use test_findex_server::start_default_test_findex_server;
use uuid::Uuid;

// TODO : should return type be string or void ?
async fn dataset_add_entries(
    rest_client: &FindexRestClient,
    index_id: &Uuid,
    entries: Vec<(Uuid, String)>,
) -> CliResult<()> {
    let _res = AddEntries {
        index_id: *index_id,
        entries,
    }
    .run(rest_client)
    .await?;

    Ok(())
}

async fn dataset_delete_entries(
    rest_client: &FindexRestClient,
    index_id: &Uuid,
    uuids: Vec<Uuid>,
) -> CliResult<()> {
    let _res = DeleteEntries {
        index_id: *index_id,
        uuids,
    }
    .run(rest_client)
    .await?;

    Ok(())
}

async fn dataset_get_entries(
    rest_client: &FindexRestClient,
    index_id: &Uuid,
    uuids: Vec<Uuid>,
) -> CliResult<EncryptedEntries> {
    GetEntries {
        index_id: *index_id,
        uuids,
    }
    .run(rest_client)
    .await
}

#[tokio::test]
#[allow(clippy::panic_in_result_fn, clippy::print_stdout)]
pub(crate) async fn test_datasets() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server().await;
    let owner_conf = FindexClientConfig::load(Some(PathBuf::from(&ctx.owner_client_conf_path)))?;
    let owner_rest_client = FindexRestClient::new(owner_conf)?;

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

    // Add entries to the dataset
    dataset_add_entries(&owner_rest_client, &index_id, encrypted_entries.clone()).await?;

    // Get the added entries from the dataset
    let added_entries = dataset_get_entries(&owner_rest_client, &index_id, uuids.clone()).await?;
    assert_eq!(added_entries.len(), entries_number);

    dataset_delete_entries(
        &owner_rest_client,
        &index_id,
        added_entries.get_uuids().deref().to_owned(),
    )
    .await?;

    // Get the added entries from the dataset
    let deleted_entries = dataset_get_entries(&owner_rest_client, &index_id, uuids).await?;
    assert_eq!(deleted_entries.len(), 0);

    Ok(())
}
