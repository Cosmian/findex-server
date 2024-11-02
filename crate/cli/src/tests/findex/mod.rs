use crate::{
    actions::{
        access::GrantAccess,
        findex::{add_or_delete::AddOrDeleteAction, search::SearchAction, FindexParameters},
    },
    error::result::CliResult,
    tests::access::{create_access_cmd, grant_access_cmd},
};
use add_or_delete::add_or_delete_cmd;
use cosmian_logger::log_utils::log_init;
use search::search_cmd;
use test_findex_server::{
    start_default_test_findex_server, start_default_test_findex_server_with_cert_auth, TestsContext,
};
use tracing::trace;

pub(crate) mod add_or_delete;
pub(crate) mod search;

#[allow(clippy::panic_in_result_fn)]
fn findex(ctx: &TestsContext, index_id: &str) -> CliResult<()> {
    // todo(manu): rename index_id to zone (or something else)
    add_or_delete_cmd(
        &ctx.owner_client_conf_path,
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

    let search_results = search_cmd(
        &ctx.owner_client_conf_path,
        SearchAction {
            findex_parameters: FindexParameters {
                key: "11223344556677889900AABBCCDDEEFF".to_owned(),
                label: "My Findex label".to_owned(),
                index_id: index_id.to_owned(),
            },
            keyword: vec!["Southborough".to_owned(), "Northbridge".to_owned()],
        },
    )?;
    assert!(search_results.contains("States9686")); // for Southborough
    assert!(search_results.contains("States14061")); // for Northbridge

    add_or_delete_cmd(
        &ctx.owner_client_conf_path,
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

    let search_results = search_cmd(
        &ctx.owner_client_conf_path,
        SearchAction {
            findex_parameters: FindexParameters {
                key: "11223344556677889900AABBCCDDEEFF".to_owned(),
                label: "My Findex label".to_owned(),
                index_id: index_id.to_owned(),
            },
            keyword: vec!["Southborough".to_owned(), "Northbridge".to_owned()],
        },
    )?;
    assert!(!search_results.contains("States9686")); // for Southborough
    assert!(!search_results.contains("States14061")); // for Northbridge

    Ok(())
}
#[tokio::test]
pub(crate) async fn test_findex_no_auth() -> CliResult<()> {
    log_init(None);
    findex(start_default_test_findex_server().await, "my_owned_index")?;
    Ok(())
}

#[tokio::test]
pub(crate) async fn test_findex_cert_auth() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server_with_cert_auth().await;

    let index_id = create_access_cmd(&ctx.owner_client_conf_path)?;
    trace!("zone: {index_id}");

    findex(ctx, &index_id)?;
    Ok(())
}

#[tokio::test]
pub(crate) async fn test_findex_grant_access() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server_with_cert_auth().await;

    let index_id = create_access_cmd(&ctx.owner_client_conf_path)?;
    trace!("index_id: {index_id}");

    grant_access_cmd(
        &ctx.owner_client_conf_path,
        GrantAccess {
            user: "owner.client@acme.com".to_owned(),
            index_id: index_id.clone(),
            role: "admin".to_owned(),
        },
    )?;

    findex(ctx, &index_id)?;
    Ok(())
}
