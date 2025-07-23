use std::{collections::HashSet, ops::Deref};

use super::Value;

#[derive(Debug)]
pub struct SearchResults(pub HashSet<Value>);

impl Deref for SearchResults {
    type Target = HashSet<Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for SearchResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut results: Vec<String> = self
            .0
            .iter()
            .filter_map(|v| String::from_utf8(v.as_ref().to_vec()).ok())
            .collect();
        results.sort();
        write!(f, "{results:?}")
    }
}
