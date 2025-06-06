//! # Findex Database Abstraction Layer
//!
//! This module provides a unified interface to different database backends for the Findex server.
//! It implements an abstraction layer that allows the application to work with any implemented DB backend.
//! and use databases interchangeably through a common API defined by various traits.
use async_trait::async_trait;
use cosmian_findex_memories::reexport::cosmian_findex::{Address, MemoryADT};
use cosmian_findex_structs::{
    CUSTOM_WORD_LENGTH, EncryptedEntries, Permission, Permissions, SERVER_ADDRESS_LENGTH, Uuids,
};
use uuid::Uuid;

use super::{
    database_traits::{DatabaseTraits, DatasetsTrait, InstantiationTrait, PermissionsTrait},
    error::DatabaseError,
    redis::Redis,
    sqlite::Sqlite,
};
use crate::config::DatabaseType;

pub(crate) type DatabaseResult<R> = Result<R, DatabaseError>;

/// A generic database enum that englobe the database backends that Findex server can use.
pub(crate) enum FindexDatabase<const WORD_LENGTH: usize> {
    Redis(Redis<WORD_LENGTH>),
    Sqlite(Sqlite<WORD_LENGTH>),
}

macro_rules! delegate_to_db {
        ($self:expr, $method:ident $(, $arg:expr)*) => {
            match $self {
                Self::Redis(redis) => redis.$method($($arg),*).await,
                Self::Sqlite(sqlite) => sqlite.$method($($arg),*).await,
            }
        };
    }

#[async_trait]
impl DatabaseTraits for FindexDatabase<CUSTOM_WORD_LENGTH> {}

#[async_trait]
impl PermissionsTrait for FindexDatabase<CUSTOM_WORD_LENGTH> {
    async fn create_index_id(&self, user_id: &str) -> DatabaseResult<Uuid> {
        delegate_to_db!(self, create_index_id, user_id)
    }

    async fn get_permissions(&self, user_id: &str) -> DatabaseResult<Permissions> {
        delegate_to_db!(self, get_permissions, user_id)
    }

    async fn get_permission(&self, user_id: &str, index_id: &Uuid) -> DatabaseResult<Permission> {
        delegate_to_db!(self, get_permission, user_id, index_id)
    }

    async fn set_permission(
        &self,
        user_id: &str,
        permission: Permission,
        index_id: &Uuid,
    ) -> DatabaseResult<()> {
        delegate_to_db!(self, set_permission, user_id, permission, index_id)
    }

    async fn revoke_permission(&self, user_id: &str, index_id: &Uuid) -> DatabaseResult<()> {
        delegate_to_db!(self, revoke_permission, user_id, index_id)
    }
}

#[async_trait]
impl DatasetsTrait for FindexDatabase<CUSTOM_WORD_LENGTH> {
    async fn dataset_add_entries(
        &self,
        index_id: &Uuid,
        entries: &EncryptedEntries,
    ) -> DatabaseResult<()> {
        delegate_to_db!(self, dataset_add_entries, index_id, entries)
    }

    async fn dataset_delete_entries(&self, index_id: &Uuid, uuids: &Uuids) -> DatabaseResult<()> {
        delegate_to_db!(self, dataset_delete_entries, index_id, uuids)
    }

    async fn dataset_get_entries(
        &self,
        index_id: &Uuid,
        uuids: &Uuids,
    ) -> DatabaseResult<EncryptedEntries> {
        delegate_to_db!(self, dataset_get_entries, index_id, uuids)
    }
}

#[async_trait]
impl<const WORD_LENGTH: usize> InstantiationTrait for FindexDatabase<WORD_LENGTH> {
    async fn instantiate(
        db_type: DatabaseType,
        db_url: &str,
        clear_database: bool,
    ) -> DatabaseResult<Self> {
        match db_type {
            DatabaseType::Redis => {
                let redis = Redis::instantiate(DatabaseType::Redis, db_url, clear_database).await?;
                Ok(Self::Redis(redis))
            }
            DatabaseType::Sqlite => {
                let sqlite =
                    Sqlite::instantiate(DatabaseType::Sqlite, db_url, clear_database).await?;
                Ok(Self::Sqlite(sqlite))
            }
        }
    }
}

impl<const WORD_LENGTH: usize> MemoryADT for FindexDatabase<WORD_LENGTH> {
    // Define the associated types required by the MemoryADT trait
    type Address = Address<SERVER_ADDRESS_LENGTH>;
    type Error = DatabaseError;
    type Word = [u8; WORD_LENGTH];

    async fn batch_read(
        &self,
        addresses: Vec<Self::Address>,
    ) -> Result<Vec<Option<Self::Word>>, Self::Error> {
        match self {
            Self::Redis(redis) => redis
                .batch_read(addresses)
                .await
                .map_err(DatabaseError::RedisFindexMemoryError),
            Self::Sqlite(sqlite) => sqlite
                .batch_read(addresses)
                .await
                .map_err(DatabaseError::SqliteFindexMemoryError),
        }
    }

    async fn guarded_write(
        &self,
        guard: (Self::Address, Option<Self::Word>),
        bindings: Vec<(Self::Address, Self::Word)>,
    ) -> Result<Option<Self::Word>, Self::Error> {
        match self {
            Self::Redis(redis) => redis
                .guarded_write(guard, bindings)
                .await
                .map_err(DatabaseError::RedisFindexMemoryError),
            Self::Sqlite(sqlite) => sqlite
                .guarded_write(guard, bindings)
                .await
                .map_err(DatabaseError::SqliteFindexMemoryError),
        }
    }
}
