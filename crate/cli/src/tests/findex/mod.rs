use crate::{
    actions::{
        access::{GrantAccess, RevokeAccess},
        findex::{add_or_delete::AddOrDeleteAction, search::SearchAction, FindexParameters},
    },
    error::result::CliResult,
    tests::access::{create_access_cmd, grant_access_cmd, revoke_access_cmd},
};
use add_or_delete::add_or_delete_cmd;
use cosmian_logger::log_utils::log_init;
use search::search_cmd;
use test_findex_server::{
    start_default_test_findex_server, start_default_test_findex_server_with_cert_auth,
};
use tracing::trace;

pub(crate) mod add_or_delete;
pub(crate) mod search;

fn add(cli_conf_path: &str, index_id: &str) -> CliResult<()> {
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

fn delete(cli_conf_path: &str, index_id: &str) -> CliResult<()> {
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

fn search(cli_conf_path: &str, index_id: &str) -> CliResult<String> {
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
fn findex(cli_conf_path: &str, index_id: &str) -> CliResult<()> {
    // todo(manu): rename index_id to zone (or something else)
    add(cli_conf_path, index_id)?;

    let search_results = search(cli_conf_path, index_id)?;
    assert!(search_results.contains("States9686")); // for Southborough
    assert!(search_results.contains("States14061")); // for Northbridge

    delete(cli_conf_path, index_id)?;

    let search_results = search(cli_conf_path, index_id)?;
    assert!(!search_results.contains("States9686")); // for Southborough
    assert!(!search_results.contains("States14061")); // for Northbridge

    Ok(())
}

#[tokio::test]
pub(crate) async fn test_findex_no_auth() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server().await;
    findex(&ctx.owner_client_conf_path, "my_owned_index")?;
    Ok(())
}

#[tokio::test]
pub(crate) async fn test_findex_cert_auth() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server_with_cert_auth().await;

    let index_id = create_access_cmd(&ctx.owner_client_conf_path)?;
    trace!("zone: {index_id}");

    findex(&ctx.owner_client_conf_path, &index_id)?;
    Ok(())
}

#[allow(clippy::panic_in_result_fn, clippy::unwrap_used)]
#[tokio::test]
pub(crate) async fn test_findex_grant_read_access() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server_with_cert_auth().await;

    let index_id = create_access_cmd(&ctx.owner_client_conf_path)?;
    trace!("index_id: {index_id}");

    add(&ctx.owner_client_conf_path, &index_id)?;

    // Grant read access to the client
    grant_access_cmd(
        &ctx.owner_client_conf_path,
        GrantAccess {
            user: "user.client@acme.com".to_owned(),
            index_id: index_id.clone(),
            role: "reader".to_owned(),
        },
    )?;

    // User can read...
    let search_results = search(&ctx.user_client_conf_path, &index_id)?;
    assert!(search_results.contains("States9686")); // for Southborough
    assert!(search_results.contains("States14061")); // for Northbridge

    // ... but not write
    assert!(add(&ctx.user_client_conf_path, &index_id).is_err());

    // Grant write access
    grant_access_cmd(
        &ctx.owner_client_conf_path,
        GrantAccess {
            user: "user.client@acme.com".to_owned(),
            index_id: index_id.clone(),
            role: "writer".to_owned(),
        },
    )?;

    // User can read...
    let search_results = search(&ctx.user_client_conf_path, &index_id)?;
    assert!(search_results.contains("States9686")); // for Southborough
    assert!(search_results.contains("States14061")); // for Northbridge

    // ... and write
    add(&ctx.user_client_conf_path, &index_id)?;

    // Try to escalade privileges from `reader` to `admin`
    grant_access_cmd(
        &ctx.user_client_conf_path,
        GrantAccess {
            user: "user.client@acme.com".to_owned(),
            index_id: index_id.clone(),
            role: "admin".to_owned(),
        },
    )
    .unwrap_err();

    revoke_access_cmd(
        &ctx.owner_client_conf_path,
        RevokeAccess {
            user: "user.client@acme.com".to_owned(),
            index_id: index_id.clone(),
        },
    )?;

    search(&ctx.user_client_conf_path, &index_id).unwrap_err();

    Ok(())
}

#[allow(clippy::panic_in_result_fn)]
#[tokio::test]
pub(crate) async fn test_findex_no_access() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server_with_cert_auth().await;

    assert!(findex(&ctx.user_client_conf_path, "whatever").is_err());
    Ok(())
}

// todo(manu):
//  - grant_access twice
