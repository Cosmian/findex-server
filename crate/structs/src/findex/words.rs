use crate::StructsError;
use base64::Engine;
use base64::engine::general_purpose;
use cosmian_crypto_core::bytes_ser_de::{Deserializer, Serializer};
use std::ops::Deref;

use super::SerializationResult;

/// Returns a `SerializationError` if any step of the serialization process fails.
fn ser_optional_word<const WORD_LENGTH: usize>(
    ser: &mut Serializer,
    word: Option<&[u8; WORD_LENGTH]>,
) -> SerializationResult<usize> {
    match word {
        Some(w) => {
            ser.write_leb128_u64(1)?;
            ser.write_array(w)
        }
        None => ser.write_leb128_u64(0),
    }
    .map_err(|e| StructsError::SerializationError(e.to_string()))
}

/// Deserializes an optional word from a deserializer.
///
/// The deserialization protocol is as follows:
/// 1. A flag is read from the deserializer.
/// 2. If the flag is `0`, `None` is returned.
/// 3. If the flag is `1`, a word is read from the deserializer and returned as `Some`.
///
/// # Errors
///
/// Returns a `DeserializationError` if any step of the deserialization process fails.
fn deser_optional_word<const WORD_LENGTH: usize>(
    de: &mut Deserializer,
) -> SerializationResult<Option<[u8; WORD_LENGTH]>> {
    let flag = <usize>::try_from(de.read_leb128_u64()?)?;
    match flag {
        0 => Ok(None),
        1 => {
            let word: [u8; WORD_LENGTH] = de.read_array()?;
            Ok(Some(word))
        }
        _ => Err(StructsError::DeserializationError(
            "Invalid value for serialized option flag".to_owned(),
        )),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionalWords<const WORD_LENGTH: usize>(pub Vec<Option<[u8; WORD_LENGTH]>>);

impl<const WORD_LENGTH: usize> From<OptionalWords<WORD_LENGTH>> for Vec<Option<[u8; WORD_LENGTH]>> {
    fn from(words: OptionalWords<WORD_LENGTH>) -> Self {
        words.0
    }
}

impl<const WORD_LENGTH: usize> From<&[Option<[u8; WORD_LENGTH]>]> for OptionalWords<WORD_LENGTH> {
    fn from(words: &[Option<[u8; WORD_LENGTH]>]) -> Self {
        Self(words.to_vec())
    }
}

impl<const WORD_LENGTH: usize> Deref for OptionalWords<WORD_LENGTH> {
    type Target = Vec<Option<[u8; WORD_LENGTH]>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const WORD_LENGTH: usize> std::fmt::Display for OptionalWords<WORD_LENGTH> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let base64_words: Vec<String> = self
            .0
            .iter()
            .map(|word| {
                word.as_ref().map_or_else(
                    || "None".to_owned(),
                    |w| general_purpose::STANDARD.encode(w),
                )
            })
            .collect();
        write!(f, "{base64_words:?}")
    }
}

impl<const WORD_LENGTH: usize> OptionalWords<WORD_LENGTH> {
    /// Creates a new `OptionalWords` instance.
    #[must_use]
    pub const fn new(words: Vec<Option<[u8; WORD_LENGTH]>>) -> Self {
        Self(words)
    }

    #[must_use]
    pub fn into_inner(self) -> Vec<Option<[u8; WORD_LENGTH]>> {
        self.0
    }

    /// Serializes the `OptionalWords` instance into a vector of bytes.
    ///
    /// # Errors
    ///
    /// This function will return an error if the serialization process fails.
    pub fn serialize(&self) -> SerializationResult<Vec<u8>> {
        if self.0.len() > 1024 {
            println!("OptionalWords: serialize: {}", self.0.len());
        }

        let mut ser = Serializer::with_capacity(self.0.len());
        ser.write_leb128_u64(self.0.len().try_into().map_err(|e| {
            StructsError::SerializationError(format!(
                "Length conversion failed. Original error : {e}"
            ))
        })?)?;
        for word in &self.0 {
            ser_optional_word(&mut ser, word.as_ref())
                .map_err(|e| StructsError::SerializationError(e.to_string()))?;
        }
        Ok(ser.finalize().to_vec())
    }

    /// Deserializes a vector of bytes into an `OptionalWords` instance.
    ///
    /// # Errors
    ///
    /// This function will return an error if the deserialization process fails.
    pub fn deserialize(data: &[u8]) -> SerializationResult<Self> {
        let mut de = Deserializer::new(data);
        let length = <usize>::try_from(de.read_leb128_u64()?)?;
        if length > 1024 {
            println!("OptionalWords: deserialize: {length}");
        }
        let mut items = Vec::with_capacity(length);

        for _ in 0..length {
            let word = deser_optional_word::<WORD_LENGTH>(&mut de)?;
            items.push(word);
        }
        Ok(Self(items))
    }
}
