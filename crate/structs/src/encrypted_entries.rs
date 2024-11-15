use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Deref, DerefMut},
};

use base64::{engine::general_purpose, Engine};
use cloudproof_findex::reexport::cosmian_crypto_core::bytes_ser_de::{Deserializer, Serializer};
use uuid::Uuid;

use crate::error::result::StructsResult;

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
            writeln!(f, "Index ID: {index_id}, Entry: {entry_b64}")?;
        }
        Ok(())
    }
}

impl Default for EncryptedEntries {
    fn default() -> Self {
        Self::new()
    }
}

impl EncryptedEntries {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn serialize(&self) -> StructsResult<Vec<u8>> {
        let mut ser = Serializer::with_capacity(self.len());
        ser.write_leb128_u64(self.len() as u64)?;
        for (uid, value) in self.iter() {
            ser.write_array(uid.as_bytes())?;
            ser.write_vec(value)?;
        }
        Ok(ser.finalize().to_vec())
    }

    pub fn deserialize(bytes: &[u8]) -> StructsResult<Self> {
        let mut de = Deserializer::new(bytes);
        let length = <usize>::try_from(de.read_leb128_u64()?)?;
        let mut items = HashMap::with_capacity(length);
        for _ in 0..length {
            let key = Uuid::from_bytes(de.read_array()?);
            let value = de.read_vec()?;
            items.insert(key, value);
        }
        Ok(Self::from(items))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use uuid::Uuid;

    use super::EncryptedEntries;

    #[test]
    fn test_encrypted_entries() {
        let mut entries = HashMap::new();
        entries.insert(Uuid::new_v4(), vec![1_u8, 2, 3]);
        entries.insert(Uuid::new_v4(), vec![4, 5, 6, 7]);
        let encrypted_entries = EncryptedEntries::from(entries);

        let serialized = encrypted_entries.serialize().unwrap();
        let deserialized = EncryptedEntries::deserialize(&serialized).unwrap();

        assert_eq!(encrypted_entries, deserialized);
    }
}
