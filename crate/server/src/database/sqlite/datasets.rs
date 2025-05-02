use std::{collections::HashMap, sync::Arc};

use super::{FINDEX_DATASETS_TABLE_NAME, Sqlite};
use crate::database::{database_traits::DatasetsTrait, findex_database::FDBResult};
use async_sqlite::rusqlite::params_from_iter;
use async_trait::async_trait;
use cosmian_findex_structs::{CUSTOM_WORD_LENGTH, EncryptedEntries, Uuids};
use tracing::instrument;
use uuid::Uuid;

#[async_trait]
impl DatasetsTrait for Sqlite<CUSTOM_WORD_LENGTH> {
    //
    // Dataset management
    //
    #[instrument(ret, err, skip_all, level = "trace")]
    async fn dataset_add_entries(
        &self,
        index_id: &Uuid,
        entries: &EncryptedEntries,
    ) -> FDBResult<()> {
        if entries.is_empty() {
            return Ok(());
        }
        // the borrow checker refuses to move the shared reference in the async block
        // as it might outlive this function. Cloning the values seems inevitable
        let index_id_bytes = Arc::new(index_id.as_bytes().to_vec());
        let entries = entries.entries.clone();

        self.pool
            .conn_mut(move |conn| {
                let tx = conn.transaction()?;

                tx.execute(
                    &format!(
                        "INSERT OR REPLACE INTO {} (index_id, user_id, encrypted_entry) VALUES {}",
                        FINDEX_DATASETS_TABLE_NAME,
                        vec!["(?,?,?)"; entries.len()].join(",")
                    ),
                    params_from_iter(entries.into_iter().flat_map(|(user_id, entry)| {
                        [
                            index_id_bytes.as_ref().clone(),
                            user_id.into_bytes().to_vec(),
                            entry,
                        ]
                    })),
                )?;
                tx.commit()?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    #[instrument(ret, err, skip(self), level = "trace")]
    async fn dataset_delete_entries(&self, index_id: &Uuid, ids: &Uuids) -> FDBResult<()> {
        // Create owned copies for the closure
        let index_id = index_id.clone();
        let ids_owned = (*ids).clone();

        self.pool
            .conn_mut(move |conn| {
                // let ids_owned = ids_owned.clone();
                let tx = conn.transaction()?;

                // If there are no IDs to delete, just commit and return
                if ids_owned.is_empty() {
                    tx.commit()?;
                    return Ok(());
                }

                // Build a query with placeholders for each ID
                tx.execute(
                    &format!(
                        "DELETE FROM {} WHERE (index_id, user_id) IN ({})",
                        FINDEX_DATASETS_TABLE_NAME,
                        vec!["(?,?)"; ids_owned.len()].join(",")
                    ),
                    params_from_iter(
                        ids_owned
                            .iter()
                            .flat_map(|id| [index_id.into_bytes(), id.into_bytes()]),
                    ),
                )?;

                tx.commit()?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    #[instrument(ret(Display), err, skip(self), level = "trace")]
    async fn dataset_get_entries(
        &self,
        index_id: &Uuid,
        ids: &Uuids,
    ) -> FDBResult<EncryptedEntries> {
        // Early return for empty IDs
        if ids.is_empty() {
            return Ok(EncryptedEntries::from(HashMap::<Uuid, Vec<u8>>::new()));
        }

        let index_id = index_id.clone();
        let ids = (*ids).clone();
        Ok(self.pool
        .conn(move |conn| {
            let query = format!(
                "SELECT index_id, encrypted_entry FROM {} WHERE index_id = ? AND user_id IN ({})",
                  FINDEX_DATASETS_TABLE_NAME,
                  vec!["?"; ids.len()].join(",")
            );
            let mut stmt = conn.prepare(&query)?;
            let mut params = Vec::with_capacity(1 + ids.len()); //  the index_id, then all entry_ids
            params.push(index_id.into_bytes().to_vec());
            params.extend(ids.iter().map(|id| id.into_bytes().to_vec()));
            let  rows = stmt.query_map(params_from_iter(params), |row| {
                let entry_id = Uuid::from_bytes(row.get::<_,[u8; 16]>(0)?);
                let encrypted_entry: Vec<u8> = row.get(1)?;
                Ok((entry_id, encrypted_entry))
            })?
            .collect::<Result<HashMap<_, _>, _>>()?;
            Ok(EncryptedEntries::from(rows))
        })
        .await?)
    }
}
