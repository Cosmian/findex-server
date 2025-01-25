use crate::{
    actions::findex::{
        index_or_delete::IndexOrDeleteAction, search::SearchAction, FindexParameters,
    },
    error::result::CliResult,
};
use cosmian_findex_client::FindexRestClient;
use std::path::PathBuf;
use tracing::trace;
use uuid::Uuid;

pub(crate) async fn add(
    key: String,
    index_id: &Uuid,
    dataset_path: &str,
    rest_client: &mut FindexRestClient,
) -> CliResult<()> {
    let index_action = IndexOrDeleteAction {
        findex_parameters: FindexParameters {
            key,
            index_id: *index_id,
        },
        csv: PathBuf::from(dataset_path),
    };
    let _res = index_action.add(rest_client).await?;
    trace!("Indexing of {} completed : {:?}", dataset_path, _res);
    Ok(())
}

pub(crate) async fn delete(
    key: String,
    index_id: &Uuid,
    dataset_path: &str,
    rest_client: &mut FindexRestClient,
) -> CliResult<()> {
    IndexOrDeleteAction {
        findex_parameters: FindexParameters {
            key,
            index_id: *index_id,
        },
        csv: PathBuf::from(dataset_path),
    }
    .delete(rest_client)
    .await?;
    trace!("Deletion of {} completed", dataset_path);
    Ok(())
}

pub(crate) struct SearchOptions {
    pub(crate) dataset_path: String,
    pub(crate) keywords: Vec<String>,
    pub(crate) expected_results: Vec<String>,
}

pub(crate) async fn search(
    key: String,
    index_id: &Uuid,
    search_options: &SearchOptions,
    rest_client: &mut FindexRestClient,
) -> CliResult<String> {
    let res = SearchAction {
        findex_parameters: FindexParameters {
            key,
            index_id: *index_id,
        },
        keyword: search_options.keywords.clone(),
    }
    .run(rest_client)
    .await?;
    trace!(
        "Search of {} completed : {:?}",
        search_options.dataset_path,
        res
    );
    Ok(res)
}
