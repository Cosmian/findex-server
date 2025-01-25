pub mod index_or_delete;
mod parameters;
pub mod search;
mod structs;

#[cfg(test)] // needed in the file crate/cli/src/tests/utils/add_delete_search.rs
pub(crate) use parameters::FindexParameters;
