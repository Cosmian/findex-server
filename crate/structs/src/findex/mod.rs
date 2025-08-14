use crate::StructsError;

pub type SerializationResult<R> = Result<R, StructsError>;

mod addresses;
mod bindings;
mod guard;
mod keywords;
mod search_results;
mod tests;
mod value;
mod words;

pub use addresses::Addresses;
pub use bindings::Bindings;
pub use guard::Guard;
pub use keywords::{Keyword, KeywordToDataSetsMap, Keywords};
pub use search_results::SearchResults;
pub use value::Value;
pub use words::{OptionalWords, Word};
