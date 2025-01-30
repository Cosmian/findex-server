// Most of this file is from legacy findex code
use cosmian_findex::Value;
use std::collections::{HashMap, HashSet};
use std::ops::{Deref, DerefMut};

/// A [`Keyword`] is a byte vector used to index other values.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Keyword(Vec<u8>);

impl<'a> From<&'a [u8]> for Keyword {
    fn from(bytes: &'a [u8]) -> Self {
        Self(bytes.to_vec())
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
            .map(|s| Keyword::from(s.as_ref()))
            .collect();
        Self(keywords)
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

impl From<Vec<&Keyword>> for Keywords {
    fn from(keywords: Vec<&Keyword>) -> Self {
        Self(keywords.into_iter().cloned().collect())
    }
}
