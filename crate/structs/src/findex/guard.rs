use cosmian_crypto_core::bytes_ser_de::{Deserializer, Serializer};
use cosmian_findex::{ADDRESS_LENGTH, Address};

use super::SerializationResult;
use crate::StructsError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Guard<const WORD_LENGTH: usize>(
    pub Address<ADDRESS_LENGTH>,
    pub Option<[u8; WORD_LENGTH]>,
);

impl<const WORD_LENGTH: usize> Guard<WORD_LENGTH> {
    /// Creates a new `Guard` instance.
    #[must_use]
    pub const fn new(address: Address<ADDRESS_LENGTH>, word: Option<[u8; WORD_LENGTH]>) -> Self {
        Self(address, word)
    }

    #[must_use]
    pub const fn into_inner(self) -> (Address<ADDRESS_LENGTH>, Option<[u8; WORD_LENGTH]>) {
        (self.0, self.1)
    }

    #[must_use]
    pub const fn possible_sizes() -> (usize, usize) {
        (ADDRESS_LENGTH + 1, ADDRESS_LENGTH + 1 + WORD_LENGTH)
    }

    /// Serializes the `Guard` instance into a vector of bytes.
    ///
    /// The serialization protocol is as follows:
    /// 1. The address is serialized as a byte array.
    /// 2. The optional word is serialized as follows:
    ///    - If the word is `Some`, a `1` is written followed by the byte array.
    ///    - If the word is `None`, a `0` is written.
    ///
    /// # Errors
    ///
    /// Returns a `SerializationError` if any step of the serialization process fails.
    pub fn serialize(&self) -> SerializationResult<Vec<u8>> {
        let mut ser = Serializer::with_capacity(ADDRESS_LENGTH + WORD_LENGTH);
        ser.write_array(self.0.as_ref())
            .map_err(|e| StructsError::SerializationError(e.to_string()))?;
        match &self.1 {
            Some(word) => {
                ser.write_leb128_u64(1)?;
                ser.write_array(word)
            }
            None => ser.write_leb128_u64(0),
        }
        .map_err(|e| StructsError::SerializationError(e.to_string()))?;
        let our_result = ser.finalize().to_vec();
        Ok(our_result)
    }

    /// Deserializes a vector of bytes into a `Guard` instance.
    ///
    /// The deserialization protocol is as follows:
    /// 1. The address is deserialized from a byte array.
    /// 2. The optional word is deserialized as follows:
    ///    - If the next byte is `1`, the word is deserialized from the following byte array.
    ///    - If the next byte is `0`, the word is set to `None`.
    ///
    /// # Errors
    ///
    /// Returns a `DeserializationError` if any step of the deserialization process fails.
    pub fn deserialize(data: &[u8]) -> SerializationResult<Self> {
        let mut de = Deserializer::new(data);
        let address: Address<ADDRESS_LENGTH> = de.read_array()?.into();
        let flag = <usize>::try_from(de.read_leb128_u64()?)?;
        let word = if flag == 1 {
            Some(de.read_array()?)
        } else if flag == 0 {
            None
        } else {
            return Err(StructsError::DeserializationError(
                "Invalid value for serialized option flag".to_owned(),
            ));
        };
        Ok(Self(address, word))
    }
}
