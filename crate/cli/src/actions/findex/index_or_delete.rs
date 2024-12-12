use std::{
    collections::{HashMap, HashSet},
    fs::File,
    path::PathBuf,
};

use clap::Parser;

use cosmian_findex::{IndexADT, Value};
use cosmian_findex_client::FindexRestClient;
use tracing::{instrument, trace};

use super::FindexParameters;
use crate::{actions::console, error::result::CliResult};
#[derive(Parser, Debug)]
#[clap(verbatim_doc_comment)]
pub struct IndexOrDeleteAction {
    #[clap(flatten)]
    pub(crate) findex_parameters: FindexParameters,

    /// The path to the CSV file containing the data to index
    #[clap(long)]
    pub(crate) csv: PathBuf,
}

/// BEGINNING OF COPY-PASTE
use std::ops::{Deref, DerefMut};

/// //! Structures used by `FindexGraph`.
use std::fmt::Display;

/// A [`Keyword`] is a byte vector used to index other values.
#[must_use]
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Keyword(Vec<u8>);

/// Implements the functionalities of a byte-vector.
///
/// # Parameters
///
/// - `type_name`   : name of the byte-vector type
macro_rules! impl_byte_vector {
    ($type_name:ty) => {
        impl AsRef<[u8]> for $type_name {
            fn as_ref(&self) -> &[u8] {
                &self.0
            }
        }

        impl Deref for $type_name {
            type Target = [u8];

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl DerefMut for $type_name {
            fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
                &mut self.0
            }
        }

        impl<'a> From<&'a [u8]> for $type_name {
            fn from(bytes: &'a [u8]) -> Self {
                Self(bytes.to_vec())
            }
        }

        impl From<Vec<u8>> for $type_name {
            fn from(bytes: Vec<u8>) -> Self {
                Self(bytes)
            }
        }

        impl From<&str> for $type_name {
            fn from(bytes: &str) -> Self {
                bytes.as_bytes().into()
            }
        }

        impl From<$type_name> for Vec<u8> {
            fn from(var: $type_name) -> Self {
                var.0
            }
        }

        impl std::fmt::Display for $type_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", String::from_utf8_lossy(&self.0))
            }
        }
    };
}

impl_byte_vector!(Keyword);

/// A [`Data`] is an arbitrary byte-string that is indexed under some keyword.
///
/// In a typical use case, it would represent a database UID and would be indexed under the
/// keywords associated to the corresponding database value.
#[must_use]
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Data(Value);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Keywords(HashSet<Keyword>);

impl Deref for Keywords {
    type Target = HashSet<Keyword>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Keywords {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromIterator<&'static str> for Keywords {
    fn from_iter<T: IntoIterator<Item = &'static str>>(iter: T) -> Self {
        Self(HashSet::from_iter(iter.into_iter().map(Keyword::from)))
    }
}

impl FromIterator<Keyword> for Keywords {
    fn from_iter<T: IntoIterator<Item = Keyword>>(iter: T) -> Self {
        Self(HashSet::from_iter(iter))
    }
}

impl IntoIterator for Keywords {
    type IntoIter = <<Self as Deref>::Target as IntoIterator>::IntoIter;
    type Item = Keyword;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Display for Keywords {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Keywords: [")?;
        for keyword in &self.0 {
            writeln!(f, "  {keyword},")?;
        }
        writeln!(f, "]")
    }
}

impl From<HashSet<Keyword>> for Keywords {
    fn from(set: HashSet<Keyword>) -> Self {
        Self(set)
    }
}

impl From<Keywords> for HashSet<Keyword> {
    fn from(value: Keywords) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct KeywordToDataSetsMap(HashMap<Keyword, HashSet<Value>>);

impl Deref for KeywordToDataSetsMap {
    type Target = HashMap<Keyword, HashSet<Value>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for KeywordToDataSetsMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<HashMap<Keyword, HashSet<Value>>> for KeywordToDataSetsMap {
    fn from(map: HashMap<Keyword, HashSet<Value>>) -> Self {
        Self(map)
    }
}

impl From<KeywordToDataSetsMap> for HashMap<Keyword, HashSet<Value>> {
    fn from(value: KeywordToDataSetsMap) -> Self {
        value.0
    }
}

impl IndexOrDeleteAction {
    /// Converts a CSV file to a hashmap where the keys are keywords and
    /// the values are sets of indexed values (Data).
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The CSV file cannot be opened.
    /// - There is an error reading the CSV records.
    /// - There is an error converting the CSV records to the expected data
    ///   types.
    /// Reads a CSV file and maps each record to a set of keywords to HashSet<Data>.
    #[instrument(err, skip(self))]
    pub(crate) fn to_indexed_value_keywords_map(&self) -> CliResult<KeywordToDataSetsMap> {
        // Initialize a HashMap to store CSV data in memory
        let mut csv_in_memory: KeywordToDataSetsMap = KeywordToDataSetsMap(HashMap::new());

        // Open the CSV file
        let file = File::open(self.csv.clone())?;
        let mut rdr = csv::Reader::from_reader(file);

        // Iterate over each record in the CSV file
        for result in rdr.byte_records() {
            // Check for errors in reading the record
            let record = result?;

            // Convert the record into a Findex indexed value
            let indexed_value = Value::from(record.as_slice());

            // Extract keywords from the record and associate them with the indexed values
            record.iter().map(Keyword::from).for_each(|k| {
                csv_in_memory
                    .entry(k)
                    .or_insert_with(HashSet::new)
                    .insert(indexed_value.clone());
            });

            // Log the CSV line for traceability
            trace!("CSV line: {record:?}");
        }

        // Return the resulting HashMap
        Ok(csv_in_memory)
    }

    async fn add_or_delete(&self, rest_client: FindexRestClient, is_insert: bool) -> CliResult<()> {
        let bindings = self.to_indexed_value_keywords_map()?;
        let iterable_bindings = bindings.iter().map(|(k, v)| (k.clone(), v.clone()));
        let findex = rest_client
            .instantiate_findex(
                &self.findex_parameters.index_id,
                &self.findex_parameters.key,
            )
            .unwrap();
        if is_insert {
            findex.insert(iterable_bindings).await
        } else {
            findex.delete(iterable_bindings).await
        }?;
        let written_keywords = bindings.keys().collect::<Vec<_>>();
        let operation_name = if is_insert { "Indexing" } else { "Deleting" };
        trace!("{} done: keywords: {:?}", operation_name, written_keywords);

        console::Stdout::new(&format!(
            "{} done: keywords: {:?}",
            operation_name, written_keywords
        ))
        .write()?;

        Ok(())
    }

    #[allow(clippy::future_not_send)]
    /// Adds the data from the CSV file to the Findex index.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - There is an error instantiating the Findex client.
    /// - There is an error retrieving the user key or label from the Findex
    ///   parameters.
    /// - There is an error converting the CSV file to a hashmap.
    /// - There is an error adding the data to the Findex index.
    /// - There is an error writing the result to the console.
    pub async fn add(&self, rest_client: FindexRestClient) -> CliResult<()> {
        Self::add_or_delete(self, rest_client, true).await
    }

    #[allow(clippy::future_not_send)]
    /// Deletes the data from the CSV file from the Findex index.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - There is an error instantiating the Findex client.
    /// - There is an error retrieving the user key or label from the Findex
    ///   parameters.
    /// - There is an error converting the CSV file to a hashmap.
    /// - There is an error deleting the data from the Findex index.
    /// - There is an error writing the result to the console.
    pub async fn delete(&self, rest_client: FindexRestClient) -> CliResult<()> {
        Self::add_or_delete(self, rest_client, false).await
    }
}
