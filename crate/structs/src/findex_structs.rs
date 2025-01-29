// Most of this file is from legacy findex code
// TODO : verify if no useless code slipped here

use cosmian_findex::Value;
use std::collections::{HashMap, HashSet};
use std::ops::{Deref, DerefMut};

/// A [`Keyword`] is a byte vector used to index other values.
#[must_use]
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Keyword(Vec<u8>);

impl AsRef<[u8]> for Keyword {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

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

impl From<&str> for Keyword {
    fn from(bytes: &str) -> Self {
        bytes.as_bytes().into()
    }
}

impl From<Keyword> for Vec<u8> {
    fn from(var: Keyword) -> Self {
        var.0
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

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Keywords(pub Vec<Keyword>);

impl From<Vec<String>> for Keywords {
    fn from(strings: Vec<String>) -> Self {
        let keywords = strings
            .into_iter()
            .map(|s| Keyword::from(s.as_str()))
            .collect();
        Self(keywords)
    }
}
