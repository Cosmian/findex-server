use super::{FINDEX_PERMISSIONS_TABLE_NAME, Sqlite};
use crate::database::{
    DatabaseError, database_traits::PermissionsTrait, findex_database::DatabaseResult,
};
use async_sqlite::rusqlite::params;
use async_trait::async_trait;
use cosmian_findex_structs::{CUSTOM_WORD_LENGTH, Permission, Permissions};
use std::collections::HashMap;
use tracing::{instrument, trace};
use uuid::Uuid;
// CREATE TABLE IF NOT EXISTS findex.permissions (
//     user_id TEXT NOT NULL,
//     index_id BLOB NOT NULL,
//     permission INTEGER NOT NULL CHECK (permission IN (0,1,2)),
//     PRIMARY KEY (user_id, index_id)
// );

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
                conn.execute(&format!(                "INSERT INTO {FINDEX_PERMISSIONS_TABLE_NAME} (user_id, index_id, permission) VALUES (?1, ?2, ?3)",
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
            conn.execute(&format!("INSERT OR REPLACE INTO {FINDEX_PERMISSIONS_TABLE_NAME} (user_id, index_id, permission) VALUES (?1, ?2, ?3)",),
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
                let query = format!("SELECT permission FROM {FINDEX_PERMISSIONS_TABLE_NAME} WHERE user_id = ?1 AND index_id = ?2");
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
                    Err(async_sqlite::rusqlite::Error::QueryReturnedNoRows)
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
                let query =
format!(                    "SELECT index_id,permission  FROM {FINDEX_PERMISSIONS_TABLE_NAME} WHERE user_id = ?1");
                let mut stmt = conn.prepare(&query)?;

                let rows = stmt
                    .query_map(params![user_id_owned], |row| {
                        let index_id = Uuid::from_bytes(row.get::<_, [u8; 16]>(0)?);
                        let permission =
                            Permission::try_from(row.get::<_, u8>(1)?).map_err(|e| {
                                async_sqlite::rusqlite::Error::FromSqlConversionFailure(
                                    // the closure signature dicates that the error type should be
                                    // rusqlite::Error, and this mapping is the closest we
                                    // can get to the original struct error (that should never happen anyway)
                                    0,
                                    async_sqlite::rusqlite::types::Type::Integer,
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

    #[instrument(ret, err, skip(self), level = "trace")]
    async fn revoke_permission(&self, user_id: &str, index_id: &Uuid) -> DatabaseResult<()> {
        let user_id_owned = user_id.to_owned();
        let index_id_bytes = index_id.into_bytes();

        self.pool
            .conn_mut(move |conn| {
                conn.execute(
                    &format!("DELETE FROM {FINDEX_PERMISSIONS_TABLE_NAME} WHERE user_id = ?1 AND index_id = ?2",),
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

    use super::*;
    use crate::{
        config::DatabaseType,
        database::{
            FindexDatabase,
            database_traits::InstantiationTrait,
            test_utils::permission_tests::{
                concurrent_create_index_id, concurrent_set_revoke_permissions,
                create_index_id_test, get_current_test_name, nonexistent_user_and_permission_test,
                revoke_permission_test, set_and_revoke_permissions_test,
            },
        },
        generate_permission_tests,
    };
    use std::env;
    use tokio;
    use tracing::debug;

    const SQLITE_TEST_DB_URL: &str = "../../target/debug/sqlite-test.db";

    fn get_sqlite_url(sqlite_url_var_env: &str) -> String {
        env::var(sqlite_url_var_env).unwrap_or_else(|_| SQLITE_TEST_DB_URL.to_owned())
    }

    async fn setup_test_db() -> Sqlite<CUSTOM_WORD_LENGTH> {
        Sqlite::instantiate(DatabaseType::Sqlite, &get_sqlite_url("SQLITE_URL"), false)
            .await
            .expect("Test failed to instantiate Sqlite")
    }

    generate_permission_tests! {
        setup_test_db().await;
        create_index_id_test,
        set_and_revoke_permissions_test,
        revoke_permission_test,
        nonexistent_user_and_permission_test,
        concurrent_set_revoke_permissions,
    }

    #[tokio::test]
    async fn test_permissions_concurrent_create_index_id() {
        debug!("RUNNING TEST: {}", get_current_test_name());
        // This lock hungry test is likely to push his processes to starvation and make itself and the other tests fail
        // It needs a configuration with a higher `busy_timeout` in order to run on the same db as the other tests
        // without ever failing. So far, editing the `busy_timeout` is not handled in the server, but it's a good feature idea.
        concurrent_create_index_id(
            FindexDatabase::<CUSTOM_WORD_LENGTH>::instantiate(
                DatabaseType::Sqlite,
                &get_sqlite_url("SQLITE_URL"),
                false,
            )
            .await
            .unwrap(),
        )
        .await
        .unwrap_or_else(|_| panic!("Test {} failed", get_current_test_name()));
    }
}
