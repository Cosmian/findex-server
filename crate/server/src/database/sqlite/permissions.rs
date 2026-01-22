use std::collections::HashMap;

use async_trait::async_trait;
use cosmian_findex_structs::{CUSTOM_WORD_LENGTH, Permission, Permissions};
use rusqlite::params;
use tracing::{instrument, trace};
use uuid::Uuid;

use super::{FINDEX_PERMISSIONS_TABLE_NAME, Sqlite};
use crate::database::{
    DatabaseError, database_traits::PermissionsTrait, findex_database::DatabaseResult,
};

#[async_trait]
impl PermissionsTrait for Sqlite<CUSTOM_WORD_LENGTH> {
    /// Creates a new index ID and sets admin privileges.
    #[instrument(ret(Display), err, skip(self), level = "trace")]
    async fn create_index_id(&self, user_id: &str) -> DatabaseResult<Uuid> {
        let index_id = Uuid::new_v4();
        let user_id_owned = user_id.to_owned();
        let index_id_bytes = index_id.into_bytes();
        let permission = u8::from(Permission::Admin);

        self.pool
            .conn_mut(move |conn| {
                conn.execute(
                    &format!(
                        "INSERT INTO {FINDEX_PERMISSIONS_TABLE_NAME} (user_id, index_id, \
                         permission) VALUES (?1, ?2, ?3)",
                    ),
                    params![user_id_owned, index_id_bytes, permission],
                )?;
                Ok(())
            })
            .await?;

        trace!("New index with id {index_id} created for user {user_id}");
        Ok(index_id)
    }

    /// Sets a permission to a user for a specific index.
    async fn set_permission(
        &self,
        user_id: &str,
        permission: Permission,
        index_id: &Uuid,
    ) -> DatabaseResult<()> {
        let user_id_owned = user_id.to_owned();
        let index_id_bytes = index_id.into_bytes();
        let permission_value = u8::from(permission);

        self.pool
            .conn_mut(move |conn| {
                conn.execute(
                    &format!(
                        "INSERT OR REPLACE INTO {FINDEX_PERMISSIONS_TABLE_NAME} (user_id, \
                         index_id, permission) VALUES (?1, ?2, ?3)",
                    ),
                    params![user_id_owned, index_id_bytes, permission_value],
                )?;
                Ok(())
            })
            .await?;

        trace!("Set {permission:?} permission to {user_id} for index {index_id}");
        Ok(())
    }

    #[instrument(ret(Display), err, skip(self), level = "trace")]
    async fn get_permission(&self, user_id: &str, index_id: &Uuid) -> DatabaseResult<Permission> {
        let user_id_owned = user_id.to_owned();
        let index_id_bytes = index_id.into_bytes();

        let permission = self
            .pool
            .conn(move |conn| {
                let query = format!(
                    "SELECT permission FROM {FINDEX_PERMISSIONS_TABLE_NAME} WHERE user_id = ?1 \
                     AND index_id = ?2"
                );
                let mut stmt = conn.prepare(&query)?;
                let mut rows = stmt.query(params![user_id_owned, index_id_bytes])?;
                if let Some(row) = rows.next()? {
                    let permission_value: u8 = row.get(0)?;
                    Ok(Permission::try_from(permission_value).map_err(|e| {
                        DatabaseError::InvalidDatabaseResponse(format!(
                            "An invalid permission value was returned by the database. {e}"
                        ))
                    }))
                } else {
                    Err(rusqlite::Error::QueryReturnedNoRows)
                }
            })
            .await??;

        trace!("Permission for user {user_id} on index {index_id}: {permission:?}");
        Ok(permission)
    }

    #[instrument(ret(Display), err, skip(self), level = "trace")]
    async fn get_permissions(&self, user_id: &str) -> DatabaseResult<Permissions> {
        let user_id_owned = user_id.to_owned();

        let red_permissions = self
            .pool
            .conn(move |conn| {
                let query = format!(
                    "SELECT index_id,permission  FROM {FINDEX_PERMISSIONS_TABLE_NAME} WHERE \
                     user_id = ?1"
                );
                let mut stmt = conn.prepare(&query)?;

                let rows = stmt
                    .query_map(params![user_id_owned], |row| {
                        let index_id = Uuid::from_bytes(row.get::<_, [u8; 16]>(0)?);
                        let permission =
                            Permission::try_from(row.get::<_, u8>(1)?).map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    // the closure signature dictates that the error type should be
                                    // rusqlite::Error, and this mapping is the closest we
                                    // can get to the original struct error (that should never happen anyway)
                                    0,
                                    rusqlite::types::Type::Integer,
                                    Box::new(e),
                                )
                            })?;
                        Ok((index_id, permission))
                    })?
                    .collect::<Result<HashMap<_, _>, _>>()?;
                Ok(Permissions { permissions: rows })
            })
            .await?;

        trace!("User {user_id} has permission {red_permissions:?}");
        Ok(red_permissions)
    }

    #[instrument(err, skip(self), level = "trace")]
    async fn revoke_permission(&self, user_id: &str, index_id: &Uuid) -> DatabaseResult<()> {
        let user_id_owned = user_id.to_owned();
        let index_id_bytes = index_id.into_bytes();

        self.pool
            .conn_mut(move |conn| {
                conn.execute(
                    &format!(
                        "DELETE FROM {FINDEX_PERMISSIONS_TABLE_NAME} WHERE user_id = ?1 AND \
                         index_id = ?2",
                    ),
                    params![user_id_owned, index_id_bytes],
                )?;
                Ok(())
            })
            .await?;

        trace!("Revoked permission for {user_id} on index {index_id}");
        Ok(())
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::get_unwrap,
    clippy::too_many_lines
)]
mod tests {

    use cosmian_crypto_core::{
        CsRng,
        reexport::rand_core::{RngCore, SeedableRng},
    };
    use tokio;
    use tracing::debug;

    use super::*;
    use crate::{
        config::DatabaseType,
        database::{
            database_traits::InstantiationTrait,
            test_utils::permission_tests::{
                concurrent_create_index_id, concurrent_set_revoke_permissions, create_index_id,
                nonexistent_user_and_permission, revoke_permission, set_and_revoke_permissions,
            },
        },
    };

    const SQLITE_TEST_DB_URL: &str = "sqlite-test";

    // This function is used to create a new SQLite database for testing purposes.
    // In some slow filesystems, using only one database for all the tests can lead to
    // starvation issues. The starving test will consume all of its body timeout time
    // and throw a `DatabaseBusy` error.
    async fn setup_a_random_test_db() -> Sqlite<CUSTOM_WORD_LENGTH> {
        let random_db = format!(
            "{}-{}.db",
            SQLITE_TEST_DB_URL,
            CsRng::from_entropy().next_u64()
        );
        Sqlite::instantiate(DatabaseType::Sqlite, &random_db, false)
            .await
            .expect("Test failed to instantiate Sqlite")
    }

    #[tokio::test]
    async fn create_index_id_test() {
        debug!("RUNNING TEST: create_index_id");
        let db = setup_a_random_test_db().await;
        create_index_id(db)
            .await
            .unwrap_or_else(|e| panic!("Test create_index_id failed: {e:?}"));
    }

    #[tokio::test]
    async fn set_and_revoke_permissions_test() {
        debug!("RUNNING TEST: set_and_revoke_permissions");
        let db = setup_a_random_test_db().await;
        set_and_revoke_permissions(db)
            .await
            .unwrap_or_else(|e| panic!("Test set_and_revoke_permissions failed: {e:?}"));
    }

    #[tokio::test]
    async fn revoke_permission_test() {
        debug!("RUNNING TEST: revoke_permission");
        let db = setup_a_random_test_db().await;
        revoke_permission(db)
            .await
            .unwrap_or_else(|e| panic!("Test revoke_permission failed: {e:?}"));
    }

    #[tokio::test]
    async fn nonexistent_user_and_permission_test() {
        debug!("RUNNING TEST: nonexistent_user_and_permission");
        let db = setup_a_random_test_db().await;
        nonexistent_user_and_permission(db)
            .await
            .unwrap_or_else(|e| panic!("Test nonexistent_user_and_permission failed: {e:?}"));
    }

    #[tokio::test]
    async fn concurrent_set_revoke_permissions_test() {
        debug!("RUNNING TEST: concurrent_set_revoke_permissions");
        let db = setup_a_random_test_db().await;
        concurrent_set_revoke_permissions(db)
            .await
            .unwrap_or_else(|e| panic!("Test concurrent_set_revoke_permissions failed: {e:?}"));
    }

    #[tokio::test]
    async fn concurrent_create_index_id_test() {
        debug!("RUNNING TEST: concurrent_create_index_id");
        let db = setup_a_random_test_db().await;
        concurrent_create_index_id(db)
            .await
            .unwrap_or_else(|e| panic!("Test concurrent_create_index_id failed: {e:?}"));
    }
}
