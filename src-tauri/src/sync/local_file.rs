// sync/local_file.rs
//
// MVP SyncProvider: export/import of a JSON bundle to/from a local file.
// This is the stand-in for a future remote sync service.

use std::fs;
use std::path::Path;

use crate::storage::TaskStorage;
use crate::sync::bundle::ImportSummary;
use crate::sync::codec::{decode_bundle, encode_bundle};
use crate::sync::{SyncError, SyncProvider};

pub struct LocalFileSync;

impl LocalFileSync {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LocalFileSync {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncProvider for LocalFileSync {
    fn export(
        &self,
        storage: &dyn TaskStorage,
        app_version: &str,
        destination: &str,
    ) -> Result<(), SyncError> {
        let bundle = storage.export_bundle(app_version)?;
        let json = encode_bundle(&bundle)?;
        fs::write(Path::new(destination), json)
            .map_err(|e| SyncError::Io(format!("could not write {destination}: {e}")))
    }

    fn import(
        &self,
        storage: &mut dyn TaskStorage,
        source: &str,
    ) -> Result<ImportSummary, SyncError> {
        let raw = fs::read_to_string(Path::new(source))
            .map_err(|e| SyncError::Io(format!("could not read {source}: {e}")))?;
        let bundle = decode_bundle(&raw)?;
        let summary = storage.import_bundle(&bundle)?;
        Ok(summary)
    }
}
