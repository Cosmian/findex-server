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

mod encrypted_entries;
mod error;
mod findex;
mod permissions;
mod uuids;

use cosmian_findex::ADDRESS_LENGTH;
pub use encrypted_entries::EncryptedEntries;
pub use error::StructsError;
pub use findex::{
    Addresses, Bindings, Guard, Keyword, KeywordToDataSetsMap, Keywords, OptionalWords,
    SearchResults, SerializationResult,
};
pub use permissions::{Permission, Permissions};
pub use uuids::Uuids;

// UID length
pub const UID_LENGTH: usize = 16;

// Findex specializations
pub const CUSTOM_WORD_LENGTH: usize = 200;
pub const SERVER_ADDRESS_LENGTH: usize = ADDRESS_LENGTH + UID_LENGTH;
