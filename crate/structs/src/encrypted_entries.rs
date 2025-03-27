use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Deref, DerefMut},
};

use base64::{Engine, engine::general_purpose};
use cosmian_crypto_core::bytes_ser_de::{Deserializer, Serializable, Serializer, to_leb128_len};
use tracing::debug;
use uuid::Uuid;

use crate::{StructsError, Uuids};

pub(crate) const UUID_LENGTH: usize = 16;

#[derive(Debug, PartialEq, Eq)]
pub struct EncryptedEntries {
    pub entries: HashMap<Uuid, Vec<u8>>,
}

impl Deref for EncryptedEntries {
    type Target = HashMap<Uuid, Vec<u8>>;

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

impl DerefMut for EncryptedEntries {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entries
    }
}

impl FromIterator<(Uuid, Vec<u8>)> for EncryptedEntries {
    fn from_iter<T: IntoIterator<Item = (Uuid, Vec<u8>)>>(iter: T) -> Self {
        Self {
            entries: iter.into_iter().collect(),
        }
    }
}

impl From<HashMap<&Uuid, Vec<u8>>> for EncryptedEntries {
    fn from(entries: HashMap<&Uuid, Vec<u8>>) -> Self {
        Self {
            entries: entries.into_iter().map(|(k, v)| (*k, v)).collect(),
        }
    }
}
impl From<HashMap<Uuid, &[u8]>> for EncryptedEntries {
    fn from(entries: HashMap<Uuid, &[u8]>) -> Self {
        Self {
            entries: entries.into_iter().map(|(k, v)| (k, v.to_vec())).collect(),
        }
    }
}
impl From<HashMap<Uuid, Vec<u8>>> for EncryptedEntries {
    fn from(entries: HashMap<Uuid, Vec<u8>>) -> Self {
        Self {
            entries: entries.into_iter().collect(),
        }
    }
}

impl Display for EncryptedEntries {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index_id, entry) in &self.entries {
            let entry_b64 = general_purpose::STANDARD.encode(entry);
            writeln!(f, "Entry ID: {index_id}, Entry Value: {entry_b64}")?;
        }
        Ok(())
    }
}

impl Default for EncryptedEntries {
    fn default() -> Self {
        Self::new()
    }
}

impl Serializable for EncryptedEntries {
    type Error = StructsError;

    fn length(&self) -> usize {
        let entries_len = self
            .entries
            .iter()
            .map(|(k, v)| k.as_bytes().len() + v.len())
            .sum::<usize>();
        to_leb128_len(self.entries.len()) + entries_len
    }

    /// Serialize the `EncryptedEntries` struct
    ///
    /// Serialization format:
    ///
    /// +----------------------+----------------------+----------------------+
    /// | Number of Entries    | Entry 1 (UUID + Vec) | Entry 2 (UUID + Vec) |
    /// +----------------------+----------------------+----------------------+
    /// |  LEB128 encoded      | UUID (16 bytes)      | UUID (16 bytes)      |
    /// |  number of entries   | Vec length (LEB128)  | Vec length (LEB128)  |
    /// |                      | Vec data (bytes)     | Vec data (bytes)     |
    /// +----------------------+----------------------+----------------------+
    fn write(&self, ser: &mut Serializer) -> Result<usize, Self::Error> {
        let mut n = ser.write_leb128_u64(u64::try_from(self.len())?)?;

        for (uid, value) in self.iter() {
            n += ser.write_array(uid.as_bytes())?;
            n += ser.write_vec(value)?;
        }
        Ok(n)
    }

    /// Deserialize the `EncryptedEntries` struct
    fn read(de: &mut Deserializer) -> Result<Self, Self::Error> {
        let length = <usize>::try_from(de.read_leb128_u64()?)?;
        if length > 1_000_000 {
            debug!("EncryptedEntries: read: allocating {length}");
        }

        let mut items = HashMap::with_capacity(length);
        for _ in 0..length {
            let key = Uuid::from_bytes(de.read_array()?);
            let value = de.read_vec()?;
            items.insert(key, value);
        }
        Ok(Self::from(items))
    }
}

impl EncryptedEntries {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    #[must_use]
    pub fn get_uuids(&self) -> Uuids {
        Uuids::from(self.entries.keys().copied().collect::<Vec<_>>())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use cosmian_crypto_core::bytes_ser_de::Serializable;
    use uuid::Uuid;

    use super::EncryptedEntries;
    use crate::error::result::StructsResult;

    #[test]
    #[allow(clippy::panic_in_result_fn)]
    fn test_encrypted_entries() -> StructsResult<()> {
        let mut entries = HashMap::new();
        entries.insert(Uuid::new_v4(), vec![1_u8, 2, 3]);
        entries.insert(Uuid::new_v4(), vec![4, 5, 6, 7]);
        let encrypted_entries = EncryptedEntries::from(entries);

        let serialized = encrypted_entries.serialize()?;
        let deserialized = EncryptedEntries::deserialize(&serialized)?;

        assert_eq!(encrypted_entries, deserialized);
        Ok(())
    }
}
