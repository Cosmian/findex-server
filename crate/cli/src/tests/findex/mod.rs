use crate::{
    actions::findex::{index::IndexAction, search::SearchAction, FindexParameters},
    error::result::CliResult,
};
use cosmian_logger::log_utils::log_init;
use index::index_cmd;
use search::search_cmd;
use test_findex_server::start_default_test_findex_server;

pub(crate) mod index;
pub(crate) mod search;

#[tokio::test]
#[allow(clippy::needless_return, clippy::panic_in_result_fn)]
pub(crate) async fn test_findex() -> CliResult<()> {
    log_init(None);
    let ctx = start_default_test_findex_server().await;

    index_cmd(
        &ctx.owner_client_conf_path,
        IndexAction {
            findex_parameters: FindexParameters {
                key: "11223344556677889900AABBCCDDEEFF".to_owned(),
                label: "My Findex label".to_owned(),
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
            },
            word: vec!["Southborough".to_owned(), "Northbridge".to_owned()],
        },
    )?;
    assert!(search_results.contains("States9686")); // for Southborough
    assert!(search_results.contains("States14061")); // for Northbridge
    Ok(())
}
