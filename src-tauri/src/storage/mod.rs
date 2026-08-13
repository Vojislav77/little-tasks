// storage/mod.rs
//
// Clean storage interface. The concrete SQLite implementation lives in
// `sqlite.rs`. Swapping the backend (or adding encryption at rest) should
// only require a new implementation of `TaskStorage` — not changes to the
// commands, sync or UI layers.

pub mod migrations;
pub mod sqlite;

use crate::core::task::{Task, TaskList};
use crate::sync::bundle::{ImportSummary, TaskBundle};

#[derive(Debug)]
pub enum StorageError {
    Sqlite(rusqlite::Error),
    Migration(migrations::MigrationError),
    Validation(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::Sqlite(e) => write!(f, "sqlite error: {e}"),
            StorageError::Migration(e) => write!(f, "migration error: {e}"),
            StorageError::Validation(e) => write!(f, "validation error: {e}"),
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StorageError::Sqlite(e) => Some(e),
            StorageError::Migration(e) => Some(e),
            StorageError::Validation(_) => None,
        }
    }
}

impl From<rusqlite::Error> for StorageError {
    fn from(e: rusqlite::Error) -> Self {
        StorageError::Sqlite(e)
    }
}

impl From<migrations::MigrationError> for StorageError {
    fn from(e: migrations::MigrationError) -> Self {
        StorageError::Migration(e)
    }
}

impl From<crate::core::task::ValidationError> for StorageError {
    fn from(e: crate::core::task::ValidationError) -> Self {
        StorageError::Validation(e.to_string())
    }
}

/// The persistence surface used by the rest of the app.
///
/// NOTE: encryption-at-rest. The MVP stores plaintext. When encryption is
/// enabled, this same interface stays: a future `EncryptedStorage` (or a
/// wrapping layer) can transparently encrypt rows / fields before writing
/// and decrypt after reading, without touching callers.
pub trait TaskStorage {
    fn create_task_list(&mut self, list: &TaskList) -> Result<(), StorageError>;
    /// Full upsert by id: inserts or overwrites the whole task list.
    fn update_task_list(&mut self, list: &TaskList) -> Result<(), StorageError>;
    /// Returns true if a list was actually deleted (tasks cascade).
    fn delete_task_list(&mut self, id: &str) -> Result<bool, StorageError>;
    fn get_task_list(&self, id: &str) -> Result<Option<TaskList>, StorageError>;
    /// All task lists, newest `updated_at` first.
    fn list_task_lists(&self) -> Result<Vec<TaskList>, StorageError>;

    fn create_task(&mut self, task: &Task) -> Result<(), StorageError>;
    /// Full upsert by id: inserts or overwrites the whole task.
    fn update_task(&mut self, task: &Task) -> Result<(), StorageError>;
    /// Returns true if a task was actually deleted.
    fn delete_task(&mut self, id: &str) -> Result<bool, StorageError>;
    fn get_task(&self, id: &str) -> Result<Option<Task>, StorageError>;
    /// Tasks, pending first then newest `updated_at`. `None` = all lists.
    fn list_tasks(&self, list_id: Option<&str>) -> Result<Vec<Task>, StorageError>;
    /// Search across task title, link and comment. Empty query returns all
    /// tasks (same ordering as list_tasks).
    fn search_tasks(&self, query: &str) -> Result<Vec<Task>, StorageError>;

    /// Export every task list + task as a portable bundle.
    fn export_bundle(&self, app_version: &str) -> Result<TaskBundle, StorageError>;
    /// Import a bundle. Semantics: upsert by id using `updated_at`
    /// comparison — the newer record wins. Tasks whose list does not
    /// exist (and is not imported) are skipped and counted.
    fn import_bundle(&mut self, bundle: &TaskBundle) -> Result<ImportSummary, StorageError>;

    /// Read a persisted setting value by key (None when unset).
    fn get_setting(&self, key: &str) -> Result<Option<String>, StorageError>;
    /// Upsert a setting value.
    fn set_setting(&mut self, key: &str, value: &str) -> Result<(), StorageError>;
}
