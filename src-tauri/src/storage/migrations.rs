// storage/migrations.rs
//
// SQLite schema versioning + migrations.
//
// Migrations are stored as an ordered list of `(version, name, sql)`.
// Applied versions are tracked in the `schema_migrations` table so that
// future migrations simply append a new entry to MIGRATIONS.
//
// v1: task_lists + tasks tables.
// v2: key/value settings table.

use rusqlite::Connection;

pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial_tasks_schema",
        sql: r#"
            CREATE TABLE task_lists (
                id          TEXT PRIMARY KEY NOT NULL,
                title       TEXT NOT NULL,
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );

            CREATE INDEX idx_task_lists_updated_at ON task_lists (updated_at DESC);

            CREATE TABLE tasks (
                id          TEXT PRIMARY KEY NOT NULL,
                list_id     TEXT NOT NULL REFERENCES task_lists(id) ON DELETE CASCADE,
                title       TEXT NOT NULL,
                done        INTEGER NOT NULL DEFAULT 0,
                link        TEXT NOT NULL DEFAULT '',
                comment     TEXT NOT NULL DEFAULT '',
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );

            CREATE INDEX idx_tasks_list_id ON tasks (list_id);
            CREATE INDEX idx_tasks_updated_at ON tasks (updated_at DESC);
            CREATE INDEX idx_tasks_done ON tasks (done, updated_at DESC);
        "#,
    },
    Migration {
        version: 2,
        name: "add_settings_table",
        sql: r#"
            CREATE TABLE settings (
                key     TEXT PRIMARY KEY NOT NULL,
                value   TEXT NOT NULL
            );
        "#,
    },
];

pub struct Migration {
    pub version: u32,
    pub name: &'static str,
    pub sql: &'static str,
}

/// Apply all pending migrations to the given connection.
/// Safe to call multiple times (idempotent).
pub fn migrate(conn: &mut Connection) -> Result<u32, MigrationError> {
    conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version     INTEGER PRIMARY KEY NOT NULL,
            name        TEXT NOT NULL,
            applied_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
        );",
    )?;

    let tx = conn.transaction()?;
    let current = schema_version(&tx)?;
    if current > latest_version() {
        return Err(MigrationError::NewerThanBinary {
            db_version: current,
            binary_version: latest_version(),
        });
    }

    for migration in MIGRATIONS {
        if migration.version <= current {
            continue;
        }
        tx.execute_batch(migration.sql)?;
        tx.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            rusqlite::params![migration.version, migration.name],
        )?;
    }
    tx.commit()?;

    Ok(schema_version(conn)?)
}

pub fn schema_version(conn: &Connection) -> Result<u32, MigrationError> {
    let v: u32 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;
    Ok(v)
}

pub fn latest_version() -> u32 {
    MIGRATIONS.last().map(|m| m.version).unwrap_or(0)
}

#[derive(Debug)]
pub enum MigrationError {
    Sqlite(rusqlite::Error),
    /// The on-disk DB is from a newer app version than this binary supports.
    NewerThanBinary {
        db_version: u32,
        binary_version: u32,
    },
}

impl std::fmt::Display for MigrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrationError::Sqlite(e) => write!(f, "sqlite error: {e}"),
            MigrationError::NewerThanBinary {
                db_version,
                binary_version,
            } => write!(
                f,
                "database schema version {db_version} is newer than supported version \
                 {binary_version}; please upgrade this app"
            ),
        }
    }
}

impl std::error::Error for MigrationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MigrationError::Sqlite(e) => Some(e),
            MigrationError::NewerThanBinary { .. } => None,
        }
    }
}

impl From<rusqlite::Error> for MigrationError {
    fn from(e: rusqlite::Error) -> Self {
        MigrationError::Sqlite(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_db_reaches_latest_version() {
        let mut conn = Connection::open_in_memory().unwrap();
        let v = migrate(&mut conn).unwrap();
        assert_eq!(v, latest_version());
        assert!(latest_version() >= 1);
    }

    #[test]
    fn migrating_twice_is_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        let v1 = migrate(&mut conn).unwrap();
        let v2 = migrate(&mut conn).unwrap();
        assert_eq!(v1, v2);
        assert_eq!(v2, latest_version());
    }

    #[test]
    fn schema_tables_exist() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('task_lists','tasks','schema_migrations','settings')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 4);
    }

    #[test]
    fn cascade_delete_removes_tasks() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO task_lists (id, title, created_at, updated_at) VALUES ('l1','L','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tasks (id, list_id, title, done, link, comment, created_at, updated_at) VALUES ('t1','l1','x',0,'','','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute("DELETE FROM task_lists WHERE id='l1'", []).unwrap();
        let left: i64 = conn
            .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
            .unwrap();
        assert_eq!(left, 0);
    }
}
