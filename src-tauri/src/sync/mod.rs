// sync/mod.rs
//
// Sync protocol interfaces + MVP local-file implementation.
//
// A future real sync service (HTTP, end-to-end, etc.) can be dropped in by
// implementing `SyncProvider` and wiring it in `backend/commands.rs` —
// the rest of the app (storage, UI, bundle format) stays untouched.

pub mod bundle;
pub mod codec;
pub mod local_file;
pub mod validate;

use crate::storage::TaskStorage;
use crate::sync::bundle::ImportSummary;

#[derive(Debug)]
pub enum SyncError {
    Bundle(String),
    Io(String),
    Storage(crate::storage::StorageError),
    UnsupportedSchema { found: u32, supported: u32 },
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::Bundle(e) => write!(f, "invalid bundle: {e}"),
            SyncError::Io(e) => write!(f, "io error: {e}"),
            SyncError::Storage(e) => write!(f, "storage error: {e}"),
            SyncError::UnsupportedSchema { found, supported } => write!(
                f,
                "bundle schema version {found} is not supported (expected <= {supported})"
            ),
        }
    }
}

impl std::error::Error for SyncError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SyncError::Bundle(_) | SyncError::Io(_) => None,
            SyncError::Storage(e) => Some(e),
            SyncError::UnsupportedSchema { .. } => None,
        }
    }
}

impl From<crate::storage::StorageError> for SyncError {
    fn from(e: crate::storage::StorageError) -> Self {
        SyncError::Storage(e)
    }
}

/// A transport-agnostic sync source/destination.
pub trait SyncProvider: Send + Sync {
    /// Serialize all tasks + lists to a bundle and write it to `destination`.
    fn export(
        &self,
        storage: &dyn TaskStorage,
        app_version: &str,
        destination: &str,
    ) -> Result<(), SyncError>;
    /// Read a bundle from `source` and merge its tasks into storage.
    fn import(&self, storage: &mut dyn TaskStorage, source: &str) -> Result<ImportSummary, SyncError>;
}
