// sync/validate.rs
//
// Structural validation of a parsed bundle, separate from JSON parsing.
// Storage import already validates each record; this is a friendlier
// pre-check used when surfacing problems to the user.

use crate::core::task::{validate_task, validate_task_list};
use crate::sync::bundle::TaskBundle;

#[derive(Debug)]
#[allow(dead_code)]
pub struct BundleValidation {
    pub list_count: usize,
    pub task_count: usize,
    pub invalid_lists: Vec<String>,
    pub invalid_tasks: Vec<String>,
}

#[allow(dead_code)]
pub fn validate_bundle(bundle: &TaskBundle) -> BundleValidation {
    let mut invalid_lists = Vec::new();
    for record in &bundle.task_lists {
        let list: crate::core::task::TaskList = record.clone().into();
        if let Err(e) = validate_task_list(&list) {
            invalid_lists.push(format!("{}: {e}", record.id));
        }
    }
    let mut invalid_tasks = Vec::new();
    for record in &bundle.tasks {
        let task: crate::core::task::Task = record.clone().into();
        if let Err(e) = validate_task(&task) {
            invalid_tasks.push(format!("{}: {e}", record.id));
        }
    }
    BundleValidation {
        list_count: bundle.task_lists.len(),
        task_count: bundle.tasks.len(),
        invalid_lists,
        invalid_tasks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::bundle::{TaskListRecord, TaskRecord};

    fn list_rec(id: &str, title: &str) -> TaskListRecord {
        TaskListRecord {
            id: id.into(),
            title: title.into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    fn task_rec(id: &str, list_id: &str, title: &str) -> TaskRecord {
        TaskRecord {
            id: id.into(),
            list_id: list_id.into(),
            title: title.into(),
            done: false,
            link: String::new(),
            comment: String::new(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn flags_only_invalid_records() {
        let mut bad_list = list_rec("bad-list", "fine title but bad timestamp");
        bad_list.created_at = "not-a-date".into();
        let mut bad_task = task_rec("bad-task", "l1", "fine title but bad timestamp");
        bad_task.created_at = "not-a-date".into();
        let bundle = TaskBundle {
            schema_version: 1,
            app_version: "0.1.0".into(),
            exported_at: String::new(),
            task_lists: vec![list_rec("good", "ok"), bad_list],
            tasks: vec![task_rec("good-task", "l1", "ok"), bad_task],
            meta: None,
        };
        let v = validate_bundle(&bundle);
        assert_eq!(v.list_count, 2);
        assert_eq!(v.task_count, 2);
        assert_eq!(v.invalid_lists.len(), 1);
        assert_eq!(v.invalid_tasks.len(), 1);
        assert!(v.invalid_lists[0].starts_with("bad-list"));
        assert!(v.invalid_tasks[0].starts_with("bad-task"));
    }
}
