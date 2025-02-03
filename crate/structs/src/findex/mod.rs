use crate::StructsError;

pub type SerializationResult<R> = Result<R, StructsError>;

mod addresses;
mod guard;
mod keywords;
mod search_results;
mod tasks;
mod tests;
mod words;

pub use addresses::Addresses;
pub use guard::Guard;
pub use keywords::{Keyword, KeywordToDataSetsMap, Keywords};
pub use search_results::SearchResults;
pub use tasks::Tasks;
pub use words::OptionalWords;
