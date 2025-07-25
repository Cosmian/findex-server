//! The lib is mostly useful for the CLI tests but
//! since it is declared, all the modules in other Files
//! will be resolved against the lib. So everything is exported
#![deny(
    nonstandard_style,
    refining_impl_trait,
    future_incompatible,
    keyword_idents,
    let_underscore,
    // rust_2024_compatibility,
    unreachable_pub,
    unused,
    unsafe_code,
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
    clippy::similar_names,
    clippy::cargo_common_metadata,
    clippy::multiple_crate_versions,
    clippy::redundant_pub_crate,
    clippy::future_not_send,
    clippy::significant_drop_tightening
)]

pub mod config;
pub mod core;
pub mod database;
pub mod error;
pub mod findex_server;
pub mod middlewares;
pub mod routes;

#[allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]
#[cfg(test)]
mod tests;

// names of the used tables. Exposed to ease out low level db montitoring
pub use database::{
    FINDEX_DATASETS_TABLE_NAME, FINDEX_MEMORY_TABLE_NAME, FINDEX_PERMISSIONS_TABLE_NAME,
};
