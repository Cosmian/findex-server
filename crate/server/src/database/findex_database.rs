//! # Findex Database Abstraction Layer
//!
//! This module provides a unified interface to different database backends for the Findex server.
//! It implements an abstraction layer that allows the application to work with any implemented DB backend.
//! and use databases interchangeably through a common API defined by various traits.
use async_trait::async_trait;
use cosmian_findex::{Address, MemoryADT};
use cosmian_findex_structs::{
    CUSTOM_WORD_LENGTH, EncryptedEntries, Permission, Permissions, SERVER_ADDRESS_LENGTH, Uuids,
};
use uuid::Uuid;

use super::{
    database_traits::{DatabaseTraits, DatasetsTrait, InstantializationTrait, PermissionsTrait},
    error::DatabaseError,
    redis::Redis,
    sqlite::Sqlite,
};

pub(crate) type FDBResult<R> = Result<R, DatabaseError>;

/// A generic database enum that englobes the database backends that Findex server can use.
pub(crate) enum FindexDatabase<const WORD_LENGTH: usize> {
    Redis(Redis<WORD_LENGTH>),
    Sqlite(Sqlite<WORD_LENGTH>),
}

#[async_trait]
impl DatabaseTraits for FindexDatabase<CUSTOM_WORD_LENGTH> {}

#[async_trait]
impl PermissionsTrait for FindexDatabase<CUSTOM_WORD_LENGTH> {
    async fn create_index_id(&self, user_id: &str) -> FDBResult<Uuid> {
        match self {
            Self::Redis(redis) => redis.create_index_id(user_id).await,
            Self::Sqlite(sqlite) => sqlite.create_index_id(user_id).await,
        }
    }

    async fn get_permissions(&self, user_id: &str) -> FDBResult<Permissions> {
        match self {
            Self::Redis(redis) => redis.get_permissions(user_id).await,
            Self::Sqlite(sqlite) => sqlite.get_permissions(user_id).await,
        }
    }

    async fn get_permission(&self, user_id: &str, index_id: &Uuid) -> FDBResult<Permission> {
        match self {
            Self::Redis(redis) => redis.get_permission(user_id, index_id).await,
            Self::Sqlite(sqlite) => sqlite.get_permission(user_id, index_id).await,
        }
    }

    async fn set_permission(
        &self,
        user_id: &str,
        permission: Permission,
        index_id: &Uuid,
    ) -> FDBResult<()> {
        match self {
            Self::Redis(redis) => redis.set_permission(user_id, permission, index_id).await,
            Self::Sqlite(sqlite) => sqlite.set_permission(user_id, permission, index_id).await,
        }
    }

    async fn revoke_permission(&self, user_id: &str, index_id: &Uuid) -> FDBResult<()> {
        match self {
            Self::Redis(redis) => redis.revoke_permission(user_id, index_id).await,
            Self::Sqlite(sqlite) => sqlite.revoke_permission(user_id, index_id).await,
        }
    }
}

#[async_trait]
impl DatasetsTrait for FindexDatabase<CUSTOM_WORD_LENGTH> {
    async fn dataset_add_entries(
        &self,
        index_id: &Uuid,
        entries: &EncryptedEntries,
    ) -> FDBResult<()> {
        match self {
            Self::Redis(redis) => redis.dataset_add_entries(index_id, entries).await,
            Self::Sqlite(sqlite) => sqlite.dataset_add_entries(index_id, entries).await,
        }
    }

    async fn dataset_delete_entries(&self, index_id: &Uuid, uuids: &Uuids) -> FDBResult<()> {
        match self {
            Self::Redis(redis) => redis.dataset_delete_entries(index_id, uuids).await,
            Self::Sqlite(sqlite) => sqlite.dataset_delete_entries(index_id, uuids).await,
        }
    }

    async fn dataset_get_entries(
        &self,
        index_id: &Uuid,
        uuids: &Uuids,
    ) -> FDBResult<EncryptedEntries> {
        match self {
            Self::Redis(redis) => redis.dataset_get_entries(index_id, uuids).await,
            Self::Sqlite(sqlite) => sqlite.dataset_get_entries(index_id, uuids).await,
        }
    }
}

#[async_trait]
impl<const WORD_LENGTH: usize> InstantializationTrait for FindexDatabase<WORD_LENGTH> {
    async fn instantiate(db_url: &str, clear_database: bool) -> FDBResult<Self> {
        // TODO: this might need an update
        if db_url.starts_with("redis://") {
            let redis = Redis::instantiate(db_url, clear_database).await?;
            Ok(Self::Redis(redis))
        } else {
            let sqlite = Sqlite::instantiate(db_url, clear_database).await?;
            Ok(Self::Sqlite(sqlite))
        }
    }
}

impl<const WORD_LENGTH: usize> MemoryADT for FindexDatabase<WORD_LENGTH> {
    // Define the associated types required by the MemoryADT trait
    type Address = Address<SERVER_ADDRESS_LENGTH>;
    type Word = [u8; WORD_LENGTH];
    type Error = DatabaseError;

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
