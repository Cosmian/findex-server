#![deny(
    nonstandard_style,
    refining_impl_trait,
    future_incompatible,
    keyword_idents,
    let_underscore,
    unreachable_pub,
    unused,
    clippy::all,
    clippy::suspicious,
    clippy::complexity,
    clippy::perf,
    clippy::style,
    clippy::pedantic,
    clippy::cargo,
    clippy::nursery,

    // restriction lints
    clippy::unwrap_used,
    clippy::get_unwrap,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_in_result,
    clippy::assertions_on_result_states,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::renamed_function_params,
    clippy::verbose_file_reads,
    clippy::str_to_string,
    clippy::string_to_string,
    clippy::unreachable,
    clippy::as_conversions,
    clippy::print_stdout,
    clippy::empty_structs_with_brackets,
    clippy::unseparated_literal_suffix,
    clippy::map_err_ignore,
    clippy::redundant_clone,
    clippy::todo

)]
#![allow(
    clippy::module_name_repetitions,
    clippy::too_many_lines,
    clippy::cargo_common_metadata,
    clippy::multiple_crate_versions,
    clippy::redundant_pub_crate
)]

pub use config::{FindexClientConfig, FINDEX_CLI_CONF_ENV};
pub use error::{result::FindexClientResult, FindexClientError};
pub use rest_client::{handle_error, FindexRestClient};

mod config;
mod datasets;
mod error;
mod permissions;
mod rest_client;

pub mod reexport {
    pub use cosmian_findex::Secret;
    pub use cosmian_findex::KEY_LENGTH as HEX_KEY_LENGTH;
    pub use cosmian_findex_config;
    pub use cosmian_http_client;
}
