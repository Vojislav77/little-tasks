// storage/sqlite.rs
//
// SQLite-backed TaskStorage. Uses a `task_lists` + `tasks` schema
// (see migrations.rs). All text is stored as UTF-8.

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::core::task::{parse_iso8601, validate_task, validate_task_list, Task, TaskList};
use crate::storage::migrations;
use crate::storage::{StorageError, TaskStorage};
use crate::sync::bundle::{ImportSummary, TaskBundle};

pub struct SqliteStorage {
    conn: Connection,
}

fn row_to_task_list(row: &Row<'_>) -> rusqlite::Result<TaskList> {
    Ok(TaskList {
        id: row.get("id")?,
        title: row.get("title")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn row_to_task(row: &Row<'_>) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get("id")?,
        list_id: row.get("list_id")?,
        title: row.get("title")?,
        done: row.get::<_, i64>("done")? != 0,
        link: row.get("link")?,
        comment: row.get("comment")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

impl SqliteStorage {
    pub fn open(path: &std::path::Path) -> Result<Self, StorageError> {
        let mut conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        migrations::migrate(&mut conn).map_err(StorageError::Migration)?;
        Ok(Self { conn })
    }

    /// In-memory DB, used by tests and future tooling.
    #[allow(dead_code)]
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let mut conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        migrations::migrate(&mut conn).map_err(StorageError::Migration)?;
        Ok(Self { conn })
    }

    /// Raw connection access for advanced tooling.
    #[allow(dead_code)]
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    fn insert_list_row(&mut self, list: &TaskList) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO task_lists (id, title, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT (id) DO UPDATE SET \
               title = excluded.title, updated_at = excluded.updated_at",
            params![list.id, list.title, list.created_at, list.updated_at],
        )?;
        Ok(())
    }

    fn insert_task_row(&mut self, task: &Task) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO tasks (id, list_id, title, done, link, comment, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
             ON CONFLICT (id) DO UPDATE SET \
               list_id = excluded.list_id, title = excluded.title, done = excluded.done, \
               link = excluded.link, comment = excluded.comment, updated_at = excluded.updated_at",
            params![
                task.id,
                task.list_id,
                task.title,
                task.done as i64,
                task.link,
                task.comment,
                task.created_at,
                task.updated_at,
            ],
        )?;
        Ok(())
    }

    fn query_lists(&self, where_clause: &str) -> Result<Vec<TaskList>, StorageError> {
        let sql = format!(
            "SELECT id, title, created_at, updated_at FROM task_lists {where_clause} \
             ORDER BY updated_at DESC"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], row_to_task_list)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    fn list_exists(&self, id: &str) -> Result<bool, StorageError> {
        let exists: bool = self
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM task_lists WHERE id = ?1)",
                params![id],
                |row| row.get(0),
            )?;
        Ok(exists)
    }
}

impl TaskStorage for SqliteStorage {
    fn create_task_list(&mut self, list: &TaskList) -> Result<(), StorageError> {
        let validated = validate_task_list(list)?;
        let exists: bool = self
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM task_lists WHERE id = ?1)",
                params![validated.id],
                |row| row.get(0),
            )?;
        if exists {
            return Err(StorageError::Validation(
                "task list with this id already exists; use update_task_list".into(),
            ));
        }
        self.insert_list_row(&validated)
    }

    fn update_task_list(&mut self, list: &TaskList) -> Result<(), StorageError> {
        let validated = validate_task_list(list)?;
        self.insert_list_row(&validated)
    }

    fn delete_task_list(&mut self, id: &str) -> Result<bool, StorageError> {
        let deleted = self
            .conn
            .execute("DELETE FROM task_lists WHERE id = ?1", params![id])?;
        Ok(deleted > 0)
    }

    fn get_task_list(&self, id: &str) -> Result<Option<TaskList>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, created_at, updated_at FROM task_lists WHERE id = ?1",
        )?;
        Ok(stmt.query_row(params![id], row_to_task_list).optional()?)
    }

    fn list_task_lists(&self) -> Result<Vec<TaskList>, StorageError> {
        self.query_lists("")
    }

    fn create_task(&mut self, task: &Task) -> Result<(), StorageError> {
        let validated = validate_task(task)?;
        if !self.list_exists(&validated.list_id)? {
            return Err(StorageError::Validation(format!(
                "task list {} does not exist",
                validated.list_id
            )));
        }
        let exists: bool = self
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM tasks WHERE id = ?1)",
                params![validated.id],
                |row| row.get(0),
            )?;
        if exists {
            return Err(StorageError::Validation(
                "task with this id already exists; use update_task".into(),
            ));
        }
        self.insert_task_row(&validated)
    }

    fn update_task(&mut self, task: &Task) -> Result<(), StorageError> {
        let validated = validate_task(task)?;
        if !self.list_exists(&validated.list_id)? {
            return Err(StorageError::Validation(format!(
                "task list {} does not exist",
                validated.list_id
            )));
        }
        self.insert_task_row(&validated)
    }

    fn delete_task(&mut self, id: &str) -> Result<bool, StorageError> {
        let deleted = self
            .conn
            .execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
        Ok(deleted > 0)
    }

    fn get_task(&self, id: &str) -> Result<Option<Task>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, list_id, title, done, link, comment, created_at, updated_at \
             FROM tasks WHERE id = ?1",
        )?;
        Ok(stmt.query_row(params![id], row_to_task).optional()?)
    }

    fn list_tasks(&self, list_id: Option<&str>) -> Result<Vec<Task>, StorageError> {
        let sql = match list_id {
            Some(_) => {
                "SELECT id, list_id, title, done, link, comment, created_at, updated_at \
                 FROM tasks WHERE list_id = ?1 ORDER BY done ASC, updated_at DESC"
            }
            None => {
                "SELECT id, list_id, title, done, link, comment, created_at, updated_at \
                 FROM tasks ORDER BY done ASC, updated_at DESC"
            }
        };
        let mut stmt = self.conn.prepare(sql)?;
        let rows = match list_id {
            Some(id) => stmt.query_map(params![id], row_to_task)?,
            None => stmt.query_map([], row_to_task)?,
        };
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    fn search_tasks(&self, query: &str) -> Result<Vec<Task>, StorageError> {
        let q = query.trim();
        if q.is_empty() {
            return self.list_tasks(None);
        }
        // Case-insensitive substring search across title, link and comment.
        let sql = r#"
            SELECT id, list_id, title, done, link, comment, created_at, updated_at
            FROM tasks
            WHERE title LIKE '%' || ?1 || '%'
               OR link   LIKE '%' || ?1 || '%'
               OR comment LIKE '%' || ?1 || '%'
            ORDER BY done ASC, updated_at DESC
        "#;
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params![q], row_to_task)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    fn export_bundle(&self, app_version: &str) -> Result<TaskBundle, StorageError> {
        let lists = self.list_task_lists()?;
        let tasks = self.list_tasks(None)?;
        Ok(TaskBundle::new(app_version, lists, tasks))
    }

    fn import_bundle(&mut self, bundle: &TaskBundle) -> Result<ImportSummary, StorageError> {
        let mut summary = ImportSummary::default();

        for record in &bundle.task_lists {
            let candidate: TaskList = record.clone().into();
            let validated = match validate_task_list(&candidate) {
                Ok(l) => l,
                Err(_) => {
                    summary.skipped_invalid += 1;
                    continue;
                }
            };
            match self.get_task_list(&validated.id)? {
                None => {
                    self.insert_list_row(&validated)?;
                    summary.imported_lists += 1;
                }
                Some(existing) => {
                    let incoming_ts = parse_iso8601(&validated.updated_at).unwrap_or(i64::MIN);
                    let existing_ts = parse_iso8601(&existing.updated_at).unwrap_or(i64::MIN);
                    if incoming_ts > existing_ts {
                        self.insert_list_row(&validated)?;
                        summary.updated_lists += 1;
                    } else {
                        summary.skipped_newer_local += 1;
                    }
                }
            }
        }
        summary.total_lists = bundle.task_lists.len();

        for record in &bundle.tasks {
            let candidate: Task = record.clone().into();
            let validated = match validate_task(&candidate) {
                Ok(t) => t,
                Err(_) => {
                    summary.skipped_invalid += 1;
                    continue;
                }
            };
            // Tasks referencing a list that neither exists nor was imported
            // cannot be placed anywhere — skip and count.
            if !self.list_exists(&validated.list_id)? {
                summary.skipped_invalid += 1;
                continue;
            }
            match self.get_task(&validated.id)? {
                None => {
                    self.insert_task_row(&validated)?;
                    summary.imported_tasks += 1;
                }
                Some(existing) => {
                    let incoming_ts = parse_iso8601(&validated.updated_at).unwrap_or(i64::MIN);
                    let existing_ts = parse_iso8601(&existing.updated_at).unwrap_or(i64::MIN);
                    if incoming_ts > existing_ts {
                        self.insert_task_row(&validated)?;
                        summary.updated_tasks += 1;
                    } else {
                        summary.skipped_newer_local += 1;
                    }
                }
            }
        }
        summary.total_tasks = bundle.tasks.len();
        Ok(summary)
    }

    fn get_setting(&self, key: &str) -> Result<Option<String>, StorageError> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM settings WHERE key = ?1")?;
        Ok(stmt.query_row(params![key], |row| row.get(0)).optional()?)
    }

    fn set_setting(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) \
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::task::{format_iso8601_utc, now_iso8601};

    fn list(id: &str, title: &str) -> TaskList {
        TaskList {
            id: id.into(),
            title: title.into(),
            created_at: now_iso8601(),
            updated_at: now_iso8601(),
        }
    }

    fn task(id: &str, list_id: &str, title: &str) -> Task {
        Task {
            id: id.into(),
            list_id: list_id.into(),
            title: title.into(),
            done: false,
            link: String::new(),
            comment: String::new(),
            created_at: now_iso8601(),
            updated_at: now_iso8601(),
        }
    }

    fn storage() -> SqliteStorage {
        SqliteStorage::open_in_memory().unwrap()
    }

    #[test]
    fn list_crud_roundtrip() {
        let mut s = storage();
        let l = list("l1", "Work");
        s.create_task_list(&l).unwrap();
        let got = s.get_task_list("l1").unwrap().unwrap();
        assert_eq!(got.title, "Work");

        let mut edited = l.clone();
        edited.title = "Work v2".into();
        s.update_task_list(&edited).unwrap();
        assert_eq!(s.get_task_list("l1").unwrap().unwrap().title, "Work v2");

        assert!(s.delete_task_list("l1").unwrap());
        assert!(!s.delete_task_list("l1").unwrap());
        assert!(s.get_task_list("l1").unwrap().is_none());
    }

    #[test]
    fn task_crud_roundtrip() {
        let mut s = storage();
        s.create_task_list(&list("l1", "Work")).unwrap();
        let t = task("t1", "l1", "Ship it");
        s.create_task(&t).unwrap();
        let got = s.get_task("t1").unwrap().unwrap();
        assert_eq!(got.title, "Ship it");
        assert_eq!(got.list_id, "l1");
        assert!(!got.done);

        let mut edited = t.clone();
        edited.done = true;
        edited.link = "https://example.com".into();
        edited.comment = "needs review".into();
        s.update_task(&edited).unwrap();
        let got = s.get_task("t1").unwrap().unwrap();
        assert!(got.done);
        assert_eq!(got.link, "https://example.com");
        assert_eq!(got.comment, "needs review");

        assert!(s.delete_task("t1").unwrap());
        assert!(!s.delete_task("t1").unwrap());
        assert!(s.get_task("t1").unwrap().is_none());
    }

    #[test]
    fn create_task_requires_existing_list() {
        let mut s = storage();
        assert!(s.create_task(&task("t1", "missing", "x")).is_err());
    }

    #[test]
    fn list_tasks_sorts_pending_first_then_newest() {
        let mut s = storage();
        s.create_task_list(&list("l1", "L")).unwrap();
        let older = Task {
            updated_at: format_iso8601_utc(1000),
            ..task("a", "l1", "old")
        };
        let newer = Task {
            updated_at: format_iso8601_utc(2000),
            ..task("b", "l1", "new")
        };
        let done_recent = Task {
            updated_at: format_iso8601_utc(3000),
            done: true,
            ..task("c", "l1", "done")
        };
        s.create_task(&older).unwrap();
        s.create_task(&newer).unwrap();
        s.create_task(&done_recent).unwrap();
        let ids: Vec<String> = s
            .list_tasks(Some("l1"))
            .unwrap()
            .into_iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(ids, vec!["b", "a", "c"]);
    }

    #[test]
    fn list_tasks_filters_by_list() {
        let mut s = storage();
        s.create_task_list(&list("l1", "A")).unwrap();
        s.create_task_list(&list("l2", "B")).unwrap();
        s.create_task(&task("t1", "l1", "one")).unwrap();
        s.create_task(&task("t2", "l2", "two")).unwrap();
        let all = s.list_tasks(None).unwrap();
        assert_eq!(all.len(), 2);
        let only_l1 = s.list_tasks(Some("l1")).unwrap();
        assert_eq!(only_l1.len(), 1);
        assert_eq!(only_l1[0].title, "one");
    }

    #[test]
    fn deleting_list_cascades_to_tasks() {
        let mut s = storage();
        s.create_task_list(&list("l1", "A")).unwrap();
        s.create_task(&task("t1", "l1", "one")).unwrap();
        s.create_task(&task("t2", "l1", "two")).unwrap();
        assert!(s.delete_task_list("l1").unwrap());
        assert_eq!(s.list_tasks(None).unwrap().len(), 0);
    }

    #[test]
    fn search_covers_title_link_and_comment() {
        let mut s = storage();
        s.create_task_list(&list("l1", "L")).unwrap();
        s.create_task(&task("a", "l1", "Alpha plans")).unwrap();
        let with_link = Task {
            link: "https://example.com/meeting".into(),
            ..task("b", "l1", "Beta")
        };
        s.create_task(&with_link).unwrap();
        let with_comment = Task {
            comment: "remember the quick brown fox".into(),
            ..task("c", "l1", "Gamma")
        };
        s.create_task(&with_comment).unwrap();

        let ids = |q: &str| {
            let mut v: Vec<String> = s.search_tasks(q).unwrap().into_iter().map(|t| t.id).collect();
            v.sort();
            v
        };
        assert_eq!(ids("alpha"), vec!["a"]);
        assert_eq!(ids("example.com"), vec!["b"]);
        assert_eq!(ids("quick brown"), vec!["c"]);
        assert_eq!(ids(""), vec!["a", "b", "c"]); // empty -> all
        assert_eq!(ids("zzz"), Vec::<String>::new());
    }

    #[test]
    fn export_import_roundtrip() {
        let mut s = storage();
        s.create_task_list(&list("l1", "Work")).unwrap();
        s.create_task_list(&list("l2", "Home")).unwrap();
        s.create_task(&task("a", "l1", "One")).unwrap();
        s.create_task(&task("b", "l2", "Two")).unwrap();
        let bundle = s.export_bundle("0.1.0").unwrap();
        assert_eq!(bundle.schema_version, 1);
        assert_eq!(bundle.task_lists.len(), 2);
        assert_eq!(bundle.tasks.len(), 2);
        assert_eq!(bundle.meta.as_ref().unwrap().task_count, 2);

        let mut other = storage();
        let summary = other.import_bundle(&bundle).unwrap();
        assert_eq!(summary.imported_lists, 2);
        assert_eq!(summary.imported_tasks, 2);
        assert_eq!(other.list_task_lists().unwrap().len(), 2);
        assert_eq!(other.list_tasks(None).unwrap().len(), 2);
    }

    #[test]
    fn import_upsert_keeps_newer_updated_at() {
        let mut local = storage();
        local.create_task_list(&list("l1", "L")).unwrap();
        let base = task("a", "l1", "old title");
        let local_task = Task {
            updated_at: format_iso8601_utc(10_000),
            title: "local newer".into(),
            ..base.clone()
        };
        local.create_task(&local_task).unwrap();

        let mut incoming = storage();
        incoming.create_task_list(&list("l1", "L")).unwrap();
        let incoming_task = Task {
            updated_at: format_iso8601_utc(5_000),
            title: "incoming older".into(),
            ..base.clone()
        };
        incoming.create_task(&incoming_task).unwrap();
        let bundle = incoming.export_bundle("0.1.0").unwrap();

        let summary = local.import_bundle(&bundle).unwrap();
        assert_eq!(summary.skipped_newer_local, 2); // list + task
        assert_eq!(local.get_task("a").unwrap().unwrap().title, "local newer");

        // Now incoming wins when it is newer.
        let mut incoming2 = storage();
        incoming2.create_task_list(&list("l1", "L")).unwrap();
        let newest = Task {
            updated_at: format_iso8601_utc(20_000),
            title: "incoming wins".into(),
            ..base.clone()
        };
        incoming2.create_task(&newest).unwrap();
        let bundle2 = incoming2.export_bundle("0.1.0").unwrap();
        let summary2 = local.import_bundle(&bundle2).unwrap();
        assert_eq!(summary2.updated_tasks, 1);
        assert_eq!(local.get_task("a").unwrap().unwrap().title, "incoming wins");
    }

    #[test]
    fn import_skips_tasks_with_missing_list() {
        use crate::sync::bundle::TaskRecord;

        let mut local = storage();
        let orphan = TaskRecord {
            id: "o".into(),
            list_id: "no-such-list".into(),
            title: "orphan".into(),
            done: false,
            link: String::new(),
            comment: String::new(),
            created_at: now_iso8601(),
            updated_at: now_iso8601(),
        };
        let good = TaskRecord {
            id: "g".into(),
            list_id: "l1".into(),
            title: "good".into(),
            done: false,
            link: String::new(),
            comment: String::new(),
            created_at: now_iso8601(),
            updated_at: now_iso8601(),
        };
        let list_rec = crate::sync::bundle::TaskListRecord {
            id: "l1".into(),
            title: "L".into(),
            created_at: now_iso8601(),
            updated_at: now_iso8601(),
        };
        let bundle = crate::sync::bundle::TaskBundle {
            schema_version: 1,
            app_version: "0.1.0".into(),
            exported_at: now_iso8601(),
            task_lists: vec![list_rec],
            tasks: vec![orphan, good],
            meta: None,
        };

        let summary = local.import_bundle(&bundle).unwrap();
        assert_eq!(summary.imported_lists, 1);
        assert_eq!(summary.imported_tasks, 1);
        assert_eq!(summary.skipped_invalid, 1);
        assert!(local.get_task("g").unwrap().is_some());
        assert!(local.get_task("o").unwrap().is_none());
    }

    #[test]
    fn settings_upsert_and_read() {
        let mut s = storage();
        assert_eq!(s.get_setting("start_with_system").unwrap(), None);
        s.set_setting("start_with_system", "1").unwrap();
        assert_eq!(s.get_setting("start_with_system").unwrap(), Some("1".into()));
        s.set_setting("start_with_system", "0").unwrap();
        assert_eq!(s.get_setting("start_with_system").unwrap(), Some("0".into()));
        assert_eq!(s.get_setting("missing").unwrap(), None);
    }
}
