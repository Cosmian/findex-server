use std::{collections::HashMap, fmt::Display};

use async_trait::async_trait;
use cloudproof_findex::{
    db_interfaces::{redis::FindexTable, rest::UpsertData},
    reexport::cosmian_findex::{
        TokenToEncryptedValueMap, TokenWithEncryptedValueList, Tokens, ENTRY_LENGTH, LINK_LENGTH,
    },
};
use uuid::Uuid;

use crate::{
    core::Permission,
    error::{result::FResult, server::FindexServerError},
};

const PERMISSION_LENGTH: usize = 1;
const INDEX_ID_LENGTH: usize = 16;

pub(crate) struct Permissions {
    pub permissions: HashMap<Uuid, Permission>,
}

impl Display for Permissions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index_id, permission) in &self.permissions {
            writeln!(f, "Index ID: {index_id}, Permission: {permission}")?;
        }
        Ok(())
    }
}

impl Permissions {
    pub(crate) fn new(index_id: Uuid, permission: Permission) -> Self {
        let mut permissions = HashMap::new();
        permissions.insert(index_id, permission);
        Self { permissions }
    }

    pub(crate) fn grant_permission(&mut self, index_id: Uuid, permission: Permission) {
        self.permissions.insert(index_id, permission);
    }

    pub(crate) fn revoke_permission(&mut self, index_id: &Uuid) {
        self.permissions.remove(index_id);
    }

    pub(crate) fn serialize(&self) -> Vec<u8> {
        let mut bytes =
            Vec::with_capacity(self.permissions.len() * (PERMISSION_LENGTH + INDEX_ID_LENGTH));
        for (index_id, permission) in &self.permissions {
            bytes.extend_from_slice(&[u8::from(permission.clone())]);
            bytes.extend_from_slice(index_id.as_bytes().as_ref());
        }
        bytes
    }

    pub(crate) fn deserialize(bytes: &[u8]) -> FResult<Self> {
        let mut permissions = HashMap::new();
        let mut i = 0;
        while i < bytes.len() {
            let permission_u8 = bytes.get(i).ok_or_else(|| {
                FindexServerError::Deserialization("Failed to deserialize Permission".to_owned())
            })?;
            let permission = Permission::try_from(*permission_u8)?;
            i += PERMISSION_LENGTH;
            let uuid_slice = bytes.get(i..i + INDEX_ID_LENGTH).ok_or_else(|| {
                FindexServerError::Deserialization(
                    "Failed to extract {INDEX_ID_LENGTH} bytes from Uuid".to_owned(),
                )
            })?;
            let index_id = Uuid::from_slice(uuid_slice).map_err(|e| {
                FindexServerError::Deserialization(format!(
                    "Failed to deserialize Uuid. Error: {e}"
                ))
            })?;
            i += INDEX_ID_LENGTH;
            permissions.insert(index_id, permission);
        }
        Ok(Self { permissions })
    }

    pub(crate) fn get_permission(&self, index_id: &Uuid) -> Option<Permission> {
        self.permissions.get(index_id).cloned()
    }
}

#[async_trait]
pub(crate) trait Database: Sync + Send {
    async fn fetch_entries(
        &self,
        index_id: &Uuid,
        tokens: Tokens,
    ) -> FResult<TokenWithEncryptedValueList<ENTRY_LENGTH>>;

    async fn fetch_chains(
        &self,
        index_id: &Uuid,
        tokens: Tokens,
    ) -> FResult<TokenWithEncryptedValueList<LINK_LENGTH>>;

    async fn upsert_entries(
        &self,
        index_id: &Uuid,
        upsert_data: UpsertData<ENTRY_LENGTH>,
    ) -> FResult<TokenToEncryptedValueMap<ENTRY_LENGTH>>;

    async fn insert_chains(
        &self,
        index_id: &Uuid,
        items: TokenToEncryptedValueMap<LINK_LENGTH>,
    ) -> FResult<()>;

    async fn delete(
        &self,
        index_id: &Uuid,
        findex_table: FindexTable,
        tokens: Tokens,
    ) -> FResult<()>;

    async fn dump_tokens(&self, index_id: &Uuid) -> FResult<Tokens>;

    async fn create_index_id(&self, user_id: &str) -> FResult<Uuid>;

    async fn get_permissions(&self, user_id: &str) -> FResult<Permissions>;

    async fn get_permission(&self, user_id: &str, index_id: &Uuid) -> FResult<Permission>;

    async fn grant_permission(
        &self,
        user_id: &str,
        permission: Permission,
        index_id: &Uuid,
    ) -> FResult<()>;

    #[allow(dead_code)]
    async fn revoke_permission(&self, user_id: &str, index_id: &Uuid) -> FResult<()>;
}
