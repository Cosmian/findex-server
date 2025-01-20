use std::path::PathBuf;

use cosmian_findex_client::FindexRestClient;
use uuid::Uuid;

use crate::{
    actions::findex::{
        index_or_delete::IndexOrDeleteAction, search::SearchAction, FindexParameters,
    },
    error::result::CliResult,
};

pub(crate) async fn add(
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
    .add(rest_client)
    .await?;
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
    Ok(res)
}
