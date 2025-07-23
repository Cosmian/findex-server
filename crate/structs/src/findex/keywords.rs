// Most of this file is from legacy findex code
use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
};

use super::Value;

/// A [`Keyword`] is a byte vector used to index other values.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Keyword(Vec<u8>);

impl<'a> From<&'a [u8]> for Keyword {
    fn from(bytes: &'a [u8]) -> Self {
        Self(bytes.to_vec())
    }
}

impl From<Vec<u8>> for Keyword {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl std::fmt::Display for Keyword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.0))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct KeywordToDataSetsMap(pub HashMap<Keyword, HashSet<Value>>);

impl Deref for KeywordToDataSetsMap {
    type Target = HashMap<Keyword, HashSet<Value>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for KeywordToDataSetsMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::fmt::Display for KeywordToDataSetsMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let base64_map: HashMap<String, Vec<String>> = self
            .0
            .iter()
            .map(|(keyword, values)| {
                (
                    String::from_utf8_lossy(&keyword.0).to_string(),
                    values
                        .iter()
                        .map(|value| String::from_utf8_lossy(value.as_ref()).to_string())
                        .collect(),
                )
            })
            .collect();
        write!(f, "{base64_map:?}")
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Keywords(pub HashSet<Keyword>);

impl From<Vec<String>> for Keywords {
    fn from(strings: Vec<String>) -> Self {
        let keywords = strings
            .into_iter()
            .map(|s| Keyword::from(s.into_bytes()))
            .collect();
        Self(keywords)
    }
}

impl From<Vec<Keyword>> for Keywords {
    fn from(keywords: Vec<Keyword>) -> Self {
        Self(keywords.into_iter().collect())
    }
}

impl std::fmt::Display for Keywords {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let base64_keywords: Vec<String> = self
            .0
            .iter()
            .map(|keyword| String::from_utf8_lossy(&keyword.0).to_string())
            .collect();
        write!(f, "{base64_keywords:?}")
    }
}
