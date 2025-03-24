use crate::StructsError;
use cosmian_crypto_core::bytes_ser_de::{Deserializer, Serializer};
use cosmian_findex::{ADDRESS_LENGTH, Address};

use super::SerializationResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bindings<const WORD_LENGTH: usize>(
    pub Vec<(Address<ADDRESS_LENGTH>, [u8; WORD_LENGTH])>,
);

impl<const WORD_LENGTH: usize> Bindings<WORD_LENGTH> {
    /// Creates a new `bindings` instance.
    #[must_use]
    pub const fn new(bindings: Vec<(Address<ADDRESS_LENGTH>, [u8; WORD_LENGTH])>) -> Self {
        Self(bindings)
    }

    #[must_use]
    pub fn into_inner(self) -> Vec<(Address<ADDRESS_LENGTH>, [u8; WORD_LENGTH])> {
        self.0
    }

    /// Serializes the `bindings` instance into a vector of bytes.
    ///
    /// The serialization protocol is as follows:
    /// 1. The length of the vector is serialized as a LEB128-encoded u64.
    /// 2. Each element in the vector is serialized as follows:
    ///    - The address is serialized as a byte array.
    ///    - The word is serialized as a byte array.
    ///
    /// # Errors
    ///
    /// Returns a `SerializationError` if any step of the serialization process fails.
    pub fn serialize(&self) -> SerializationResult<Vec<u8>> {
        let mut ser = Serializer::with_capacity(self.0.len());
        ser.write_leb128_u64(self.0.len().try_into().map_err(|e| {
            StructsError::SerializationError(format!(
                "Length conversion failed. Original error : {e}"
            ))
        })?)?;
        for (address, word) in &self.0 {
            {
                ser.write_array(address.as_ref())?;
                ser.write_array(word)
            }
            .map_err(|e| StructsError::SerializationError(e.to_string()))?;
        }
        Ok(ser.finalize().to_vec())
    }

    /// Deserializes a vector of bytes into a `bindings` instance.
    ///
    /// The deserialization protocol is as follows:
    /// 1. The length of the vector is deserialized from a LEB128-encoded u64.
    /// 2. Each element in the vector is deserialized as follows:
    ///    - The address iz deserialized from a byte array.
    ///    - The word is deserialized from a byte array.
    ///
    /// # Errors
    ///
    /// Returns a `DeserializationError` if any step of the deserialization process fails.
    pub fn deserialize(data: &[u8]) -> SerializationResult<Self> {
        let mut de = Deserializer::new(data);
        let length = <usize>::try_from(de.read_leb128_u64()?)?;
        let mut items = Vec::with_capacity(length);
        for _ in 0..length {
            let address: Address<ADDRESS_LENGTH> = de.read_array()?.into();
            let word: [u8; WORD_LENGTH] = de.read_array()?;
            items.push((address, word));
        }
        Ok(Self(items))
    }
}
