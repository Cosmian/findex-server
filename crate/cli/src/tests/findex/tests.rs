use cosmian_findex_structs::Permission;
use cosmian_logger::log_init;
use test_findex_server::{
    start_default_test_findex_server, start_default_test_findex_server_with_cert_auth,
};
use tracing::trace;
use uuid::Uuid;

use super::{index_or_delete::index_or_delete_cmd, search::search_cmd};
use crate::{
    actions::{
        findex::{index_or_delete::IndexOrDeleteAction, search::SearchAction, FindexParameters},
        permissions::{GrantPermission, ListPermissions, RevokePermission},
    },
    error::result::CliResult,
    tests::permissions::{
        create_index_id_cmd, grant_permission_cmd, list_permission_cmd, revoke_permission_cmd,
    },
};

struct SearchOptions {
    dataset_path: String,
    keywords: Vec<String>,
    expected_results: Vec<String>,
}

const SMALL_DATASET: &str = "../../test_data/datasets/smallpop.csv";
const HUGE_DATASET: &str = "../../test_data/datasets/business-employment.csv";

fn add(cli_conf_path: &str, index_id: &Uuid, dataset_path: &str) -> CliResult<()> {
    index_or_delete_cmd(
        cli_conf_path,
        "index",
        IndexOrDeleteAction {
            findex_parameters: FindexParameters {
                key: "11223344556677889900AABBCCDDEEFF11223344556677889900AABBCCDDEEFF".to_string(),
                index_id: index_id.to_owned(),
            },
            csv: dataset_path.into(),
        },
    )?;
    Ok(())
}

fn delete(cli_conf_path: &str, index_id: &Uuid, dataset_path: &str) -> CliResult<()> {
    index_or_delete_cmd(
        cli_conf_path,
        "delete",
        IndexOrDeleteAction {
            findex_parameters: FindexParameters {
                key: "11223344556677889900AABBCCDDEEFF11223344556677889900AABBCCDDEEFF".to_string(),
                index_id: index_id.to_owned(),
            },
            csv: dataset_path.into(),
        },
    )?;
    Ok(())
}

fn search(
    cli_conf_path: &str,
    index_id: &Uuid,
    search_options: &SearchOptions,
) -> CliResult<String> {
    search_cmd(
        cli_conf_path,
        SearchAction {
            findex_parameters: FindexParameters {
                key: "11223344556677889900AABBCCDDEEFF11223344556677889900AABBCCDDEEFF".to_string(),
                index_id: index_id.to_owned(),
            },
            keyword: search_options.keywords.clone(),
        },
    )
}

#[allow(clippy::panic_in_result_fn)]
fn add_search_delete(
    cli_conf_path: &str,
    index_id: &Uuid,
    search_options: &SearchOptions,
) -> CliResult<()> {
    add(cli_conf_path, index_id, &search_options.dataset_path)?;

    // make sure searching returns the expected results
    let search_results = search(cli_conf_path, index_id, search_options)?;
    for expected_result in &search_options.expected_results {
        assert!(search_results.contains(expected_result));
    }

    delete(cli_conf_path, index_id, &search_options.dataset_path)?;

    // make sure no results are returned after deletion
    let search_results = search(cli_conf_path, index_id, search_options)?;
    for expected_result in &search_options.expected_results {
        assert!(!search_results.contains(expected_result));
    }

    Ok(())
}

#[tokio::test]
pub(crate) async fn test_findex_no_auth() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server().await;

    // Search 2 entries in a small dataset. Expect 2 results.
    let search_options = SearchOptions {
        dataset_path: SMALL_DATASET.into(),
        keywords: vec!["Southborough".to_owned(), "Northbridge".to_owned()],
        expected_results: vec!["States9686".to_owned(), "States14061".to_owned()],
    };
    add_search_delete(
        &ctx.owner_client_conf_path,
        &Uuid::new_v4(),
        &search_options,
    )?;
    Ok(())
}

#[ignore]
#[tokio::test]
pub(crate) async fn test_findex_no_auth_huge_dataset() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server().await;

    // Search 1 entry in a huge dataset
    let search_options = SearchOptions {
        dataset_path: HUGE_DATASET.into(),
        keywords: vec![
            "BDCQ.SEA1AA".to_owned(),
            "2011.06".to_owned(),
            "80078".to_owned(),
        ],
        expected_results: vec![
            "80078FNumber0Business Data Collection - BDCIndustry by employment variableFilled \
             jobsAgriculture, Forestry and FishingActual'"
                .to_owned(),
        ],
    };
    add_search_delete(
        &ctx.owner_client_conf_path,
        &Uuid::new_v4(),
        &search_options,
    )?;
    Ok(())
}

#[tokio::test]
pub(crate) async fn test_findex_cert_auth() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server_with_cert_auth().await;

    let search_options = SearchOptions {
        dataset_path: SMALL_DATASET.into(),
        keywords: vec!["Southborough".to_owned(), "Northbridge".to_owned()],
        expected_results: vec!["States9686".to_owned(), "States14061".to_owned()],
    };

    let index_id = create_index_id_cmd(&ctx.owner_client_conf_path).await?;
    trace!("index_id: {index_id}");

    add_search_delete(&ctx.owner_client_conf_path, &index_id, &search_options)?;
    Ok(())
}

#[allow(clippy::panic_in_result_fn, clippy::unwrap_used)]
#[tokio::test]
pub(crate) async fn test_findex_grant_and_revoke_permission() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server_with_cert_auth().await;

    let search_options = SearchOptions {
        dataset_path: SMALL_DATASET.into(),
        keywords: vec!["Southborough".to_owned(), "Northbridge".to_owned()],
        expected_results: vec!["States9686".to_owned(), "States14061".to_owned()],
    };

    let index_id = create_index_id_cmd(&ctx.owner_client_conf_path).await?;
    trace!("index_id: {index_id}");

    add(&ctx.owner_client_conf_path, &index_id, SMALL_DATASET)?;

    // Grant read permission to the client
    grant_permission_cmd(
        &ctx.owner_client_conf_path,
        &GrantPermission {
            user: "user.client@acme.com".to_owned(),
            index_id,
            permission: Permission::Read,
        },
    )?;

    // User can read...
    let search_results = search(&ctx.user_client_conf_path, &index_id, &search_options)?;
    for expected_result in &search_options.expected_results {
        assert!(search_results.contains(expected_result));
    }

    // ... but not write
    assert!(add(&ctx.user_client_conf_path, &index_id, SMALL_DATASET).is_err());

    // Grant write permission
    grant_permission_cmd(
        &ctx.owner_client_conf_path,
        &GrantPermission {
            user: "user.client@acme.com".to_owned(),
            index_id,
            permission: Permission::Write,
        },
    )?;

    list_permission_cmd(
        &ctx.owner_client_conf_path,
        &ListPermissions {
            user: "user.client@acme.com".to_owned(),
        },
    )?;

    // User can read...
    let search_results = search(&ctx.user_client_conf_path, &index_id, &search_options)?;
    for expected_result in &search_options.expected_results {
        assert!(search_results.contains(expected_result));
    }

    // ... and write
    add(&ctx.user_client_conf_path, &index_id, SMALL_DATASET)?;

    // Try to escalade privileges from `read` to `admin`
    grant_permission_cmd(
        &ctx.user_client_conf_path,
        &GrantPermission {
            user: "user.client@acme.com".to_owned(),
            index_id,
            permission: Permission::Admin,
        },
    )
    .unwrap_err();

    revoke_permission_cmd(
        &ctx.owner_client_conf_path,
        &RevokePermission {
            user: "user.client@acme.com".to_owned(),
            index_id,
        },
    )?;

    search(&ctx.user_client_conf_path, &index_id, &search_options).unwrap_err();

    Ok(())
}

#[allow(clippy::panic_in_result_fn)]
#[tokio::test]
pub(crate) async fn test_findex_no_permission() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server_with_cert_auth().await;

    let search_options = SearchOptions {
        dataset_path: SMALL_DATASET.into(),
        keywords: vec!["Southborough".to_owned(), "Northbridge".to_owned()],
        expected_results: vec!["States9686".to_owned(), "States14061".to_owned()],
    };

    assert!(
        add_search_delete(&ctx.user_client_conf_path, &Uuid::new_v4(), &search_options).is_err()
    );
    Ok(())
}
