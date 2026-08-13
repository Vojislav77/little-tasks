// sync/bundle.rs
//
// The portable "sync bundle" JSON format — the MVP cross-device bridge.
//
// ```json
// {
//   "schema_version": 1,
//   "app_version": "0.1.0",
//   "exported_at": "2026-08-03T18:28:00Z",
//   "task_lists": [ ... TaskListRecord ... ],
//   "tasks": [ ... TaskRecord ... ],
//   "meta": { "source": "little-tasks", "list_count": 2, "task_count": 3 }
// }
// ```

use crate::core::task::{Task, TaskList};

pub const SCHEMA_VERSION: u32 = 1;

/// Wire format for a single task list inside a bundle.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskListRecord {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(alias = "created_at")]
    pub created_at: String,
    #[serde(alias = "updated_at")]
    pub updated_at: String,
}

/// Wire format for a single task inside a bundle.
/// Every field except `id`, `list_id`, `created_at`, `updated_at` is
/// optional and defaults to a safe value so older / partial exports still
/// import. `alias` accepts the old snake_case keys for robustness.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRecord {
    pub id: String,
    #[serde(alias = "list_id")]
    pub list_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default, alias = "is_done")]
    pub done: bool,
    #[serde(default)]
    pub link: String,
    #[serde(default)]
    pub comment: String,
    #[serde(alias = "created_at")]
    pub created_at: String,
    #[serde(alias = "updated_at")]
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleMeta {
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default)]
    pub list_count: usize,
    #[serde(default)]
    pub task_count: usize,
}

fn default_source() -> String {
    "little-tasks".into()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskBundle {
    #[serde(default = "default_schema_version", alias = "schema_version")]
    pub schema_version: u32,
    #[serde(default, alias = "app_version")]
    pub app_version: String,
    #[serde(default, alias = "exported_at")]
    pub exported_at: String,
    #[serde(default, alias = "task_lists")]
    pub task_lists: Vec<TaskListRecord>,
    #[serde(default, alias = "tasks")]
    pub tasks: Vec<TaskRecord>,
    #[serde(default)]
    pub meta: Option<BundleMeta>,
}

fn default_schema_version() -> u32 {
    SCHEMA_VERSION
}

impl TaskBundle {
    pub fn new(app_version: &str, lists: Vec<TaskList>, tasks: Vec<Task>) -> Self {
        let list_count = lists.len();
        let task_count = tasks.len();
        Self {
            schema_version: SCHEMA_VERSION,
            app_version: app_version.to_string(),
            exported_at: crate::core::task::now_iso8601(),
            task_lists: lists.into_iter().map(TaskListRecord::from).collect(),
            tasks: tasks.into_iter().map(TaskRecord::from).collect(),
            meta: Some(BundleMeta {
                source: default_source(),
                list_count,
                task_count,
            }),
        }
    }
}

impl From<&TaskList> for TaskListRecord {
    fn from(list: &TaskList) -> Self {
        Self {
            id: list.id.clone(),
            title: list.title.clone(),
            created_at: list.created_at.clone(),
            updated_at: list.updated_at.clone(),
        }
    }
}

impl From<TaskList> for TaskListRecord {
    fn from(list: TaskList) -> Self {
        Self::from(&list)
    }
}

impl From<TaskListRecord> for TaskList {
    fn from(record: TaskListRecord) -> Self {
        Self {
            id: record.id,
            title: record.title,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

impl From<&Task> for TaskRecord {
    fn from(task: &Task) -> Self {
        Self {
            id: task.id.clone(),
            list_id: task.list_id.clone(),
            title: task.title.clone(),
            done: task.done,
            link: task.link.clone(),
            comment: task.comment.clone(),
            created_at: task.created_at.clone(),
            updated_at: task.updated_at.clone(),
        }
    }
}

impl From<Task> for TaskRecord {
    fn from(task: Task) -> Self {
        Self::from(&task)
    }
}

impl From<TaskRecord> for Task {
    fn from(record: TaskRecord) -> Self {
        Self {
            id: record.id,
            list_id: record.list_id,
            title: record.title,
            done: record.done,
            link: record.link,
            comment: record.comment,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub total_lists: usize,
    pub imported_lists: usize,
    pub updated_lists: usize,
    pub total_tasks: usize,
    pub imported_tasks: usize,
    pub updated_tasks: usize,
    pub skipped_newer_local: usize,
    pub skipped_invalid: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_roundtrip_preserves_all_fields() {
        let task = Task {
            id: "t1".into(),
            list_id: "l1".into(),
            title: "Write docs".into(),
            done: true,
            link: "https://example.com".into(),
            comment: "with examples".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-02T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&TaskRecord::from(&task)).unwrap();
        let back: TaskRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back.title, "Write docs");
        assert!(back.done);
        assert_eq!(back.link, "https://example.com");
        assert_eq!(back.comment, "with examples");
    }

    #[test]
    fn missing_optional_fields_use_defaults() {
        let json = r#"{
            "id": "t1",
            "list_id": "l1",
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        }"#;
        let rec: TaskRecord = serde_json::from_str(json).unwrap();
        assert_eq!(rec.title, "");
        assert!(!rec.done);
        assert_eq!(rec.link, "");
        assert_eq!(rec.comment, "");
    }

    #[test]
    fn bundle_roundtrip() {
        let list = TaskList {
            id: "l1".into(),
            title: "Work".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        let task = Task {
            id: "t1".into(),
            list_id: "l1".into(),
            title: "Ship it".into(),
            done: false,
            link: String::new(),
            comment: String::new(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        let bundle = TaskBundle::new("0.1.0", vec![list], vec![task]);
        let json = serde_json::to_string(&bundle).unwrap();
        let back: TaskBundle = serde_json::from_str(&json).unwrap();
        assert_eq!(back.schema_version, SCHEMA_VERSION);
        assert_eq!(back.task_lists.len(), 1);
        assert_eq!(back.tasks.len(), 1);
        assert_eq!(back.tasks[0].list_id, "l1");
    }
}
