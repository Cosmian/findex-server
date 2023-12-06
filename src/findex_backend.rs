use cloudproof_findex::{
    backends::{
        sqlite::{SqlChainBackend, SqlEntryBackend},
        BackendError,
    },
    BackendConfiguration,
};
use cosmian_findex::DxEnc;
use cosmian_findex::{ChainTable, EntryTable, ENTRY_LENGTH, LINK_LENGTH};
pub struct SqliteFindexBackend {
    pub entry: EntryTable<ENTRY_LENGTH, SqlEntryBackend>,
    pub chain: ChainTable<LINK_LENGTH, SqlChainBackend>,
}

impl SqliteFindexBackend {
    pub fn new(config: BackendConfiguration) -> Result<Self, BackendError> {
        match config {
            BackendConfiguration::Sqlite(entry_params, chain_params) => Ok(SqliteFindexBackend {
                entry: EntryTable::setup(SqlEntryBackend::new(&entry_params)?),
                chain: ChainTable::setup(SqlChainBackend::new(&chain_params)?),
            }),
            _ => todo!(),
        }
    }
}
