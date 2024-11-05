use add_or_delete::add_or_delete_cmd;
use cosmian_logger::log_utils::log_init;
use cosmian_rest_client::Permission;
use search::search_cmd;
use test_findex_server::{
    start_default_test_findex_server, start_default_test_findex_server_with_cert_auth,
};
use tracing::trace;
use uuid::Uuid;

use crate::{
    actions::{
        findex::{add_or_delete::AddOrDeleteAction, search::SearchAction, FindexParameters},
        permissions::{GrantPermission, RevokePermission},
    },
    error::result::CliResult,
    tests::permissions::{create_index_id_cmd, grant_permission_cmd, revoke_permission_cmd},
};

pub(crate) mod add_or_delete;
pub(crate) mod search;

fn add(cli_conf_path: &str, index_id: &Uuid) -> CliResult<()> {
    add_or_delete_cmd(
        cli_conf_path,
        "add",
        AddOrDeleteAction {
            findex_parameters: FindexParameters {
                key: "11223344556677889900AABBCCDDEEFF".to_owned(),
                label: "My Findex label".to_owned(),
                index_id: index_id.to_owned(),
            },
            csv: "src/tests/datasets/smallpop.csv".into(),
        },
    )?;
    Ok(())
}

fn delete(cli_conf_path: &str, index_id: &Uuid) -> CliResult<()> {
    add_or_delete_cmd(
        cli_conf_path,
        "delete",
        AddOrDeleteAction {
            findex_parameters: FindexParameters {
                key: "11223344556677889900AABBCCDDEEFF".to_owned(),
                label: "My Findex label".to_owned(),
                index_id: index_id.to_owned(),
            },
            csv: "src/tests/datasets/smallpop.csv".into(),
        },
    )?;
    Ok(())
}

fn search(cli_conf_path: &str, index_id: &Uuid) -> CliResult<String> {
    search_cmd(
        cli_conf_path,
        SearchAction {
            findex_parameters: FindexParameters {
                key: "11223344556677889900AABBCCDDEEFF".to_owned(),
                label: "My Findex label".to_owned(),
                index_id: index_id.to_owned(),
            },
            keyword: vec!["Southborough".to_owned(), "Northbridge".to_owned()],
        },
    )
}

#[allow(clippy::panic_in_result_fn)]
fn add_search_delete(cli_conf_path: &str, index_id: &Uuid) -> CliResult<()> {
    add(cli_conf_path, index_id)?;

    // make sure searching returns the expected results
    let search_results = search(cli_conf_path, index_id)?;
    assert!(search_results.contains("States9686")); // for Southborough
    assert!(search_results.contains("States14061")); // for Northbridge

    delete(cli_conf_path, index_id)?;

    // make sure no results are returned after deletion
    let search_results = search(cli_conf_path, index_id)?;
    assert!(!search_results.contains("States9686")); // for Southborough
    assert!(!search_results.contains("States14061")); // for Northbridge

    Ok(())
}

#[tokio::test]
pub(crate) async fn test_findex_no_auth() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server().await;
    add_search_delete(&ctx.owner_client_conf_path, &Uuid::new_v4())?;
    Ok(())
}

#[tokio::test]
pub(crate) async fn test_findex_cert_auth() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server_with_cert_auth().await;

    let index_id = create_index_id_cmd(&ctx.owner_client_conf_path)?;
    trace!("index_id: {index_id}");

    add_search_delete(&ctx.owner_client_conf_path, &index_id)?;
    Ok(())
}

#[allow(clippy::panic_in_result_fn, clippy::unwrap_used)]
#[tokio::test]
pub(crate) async fn test_findex_grant_and_revoke_permission() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server_with_cert_auth().await;

    let index_id = create_index_id_cmd(&ctx.owner_client_conf_path)?;
    trace!("index_id: {index_id}");

    add(&ctx.owner_client_conf_path, &index_id)?;

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
    let search_results = search(&ctx.user_client_conf_path, &index_id)?;
    assert!(search_results.contains("States9686")); // for Southborough
    assert!(search_results.contains("States14061")); // for Northbridge

    // ... but not write
    assert!(add(&ctx.user_client_conf_path, &index_id).is_err());

    // Grant write permission
    grant_permission_cmd(
        &ctx.owner_client_conf_path,
        &GrantPermission {
            user: "user.client@acme.com".to_owned(),
            index_id,
            permission: Permission::Write,
        },
    )?;

    // User can read...
    let search_results = search(&ctx.user_client_conf_path, &index_id)?;
    assert!(search_results.contains("States9686")); // for Southborough
    assert!(search_results.contains("States14061")); // for Northbridge

    // ... and write
    add(&ctx.user_client_conf_path, &index_id)?;

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

    search(&ctx.user_client_conf_path, &index_id).unwrap_err();

    Ok(())
}

#[allow(clippy::panic_in_result_fn)]
#[tokio::test]
pub(crate) async fn test_findex_no_permission() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server_with_cert_auth().await;

    assert!(add_search_delete(&ctx.user_client_conf_path, &Uuid::new_v4()).is_err());
    Ok(())
}
