//! A generic byte container for Findex data values.
//!
//! This module provides the [`Value`] struct, which serves as a type-safe wrapper
//! around byte vectors for storing the values returned upon searching with Findex.

use std::string::FromUtf8Error;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Value(Vec<u8>);

impl AsRef<[u8]> for Value {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<&[u8]> for Value {
    fn from(value: &[u8]) -> Self {
        Self(value.to_vec())
    }
}

impl From<Vec<u8>> for Value {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl From<&Value> for Vec<u8> {
    fn from(value: &Value) -> Self {
        value.0.clone()
    }
}

impl From<Value> for Vec<u8> {
    fn from(value: Value) -> Self {
        value.0
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self(value.as_bytes().to_vec())
    }
}

impl TryFrom<Value> for String {
    type Error = FromUtf8Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        Self::from_utf8(value.0)
    }
}

impl From<usize> for Value {
    fn from(value: usize) -> Self {
        Self(value.to_be_bytes().to_vec())
    }
}
