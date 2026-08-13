// sync/codec.rs
//
// JSON encoding/decoding for bundles. Decoding is deliberately lenient:
// it tolerates missing optional fields (via serde defaults) and reports
// a friendly error instead of panicking on malformed input.

use crate::sync::bundle::{TaskBundle, SCHEMA_VERSION};
use crate::sync::SyncError;

pub fn encode_bundle(bundle: &TaskBundle) -> Result<String, SyncError> {
    serde_json::to_string_pretty(bundle)
        .map_err(|e| SyncError::Bundle(format!("serialization failed: {e}")))
}

/// Parse JSON into a bundle and validate the schema version.
/// Returns `SyncError` on malformed JSON, unknown shapes, or an
/// unsupported schema version — callers surface this to the user
/// as an error dialog rather than crashing.
pub fn decode_bundle(json: &str) -> Result<TaskBundle, SyncError> {
    let parsed: serde_json::Value =
        serde_json::from_str(json).map_err(|e| SyncError::Bundle(format!("not valid JSON: {e}")))?;

    if !parsed.is_object() {
        return Err(SyncError::Bundle(
            "expected a JSON object at the top level".into(),
        ));
    }

    let bundle: TaskBundle = serde_json::from_value(parsed).map_err(|e| {
        SyncError::Bundle(format!("bundle is missing required fields: {e}"))
    })?;

    if bundle.schema_version > SCHEMA_VERSION {
        return Err(SyncError::UnsupportedSchema {
            found: bundle.schema_version,
            supported: SCHEMA_VERSION,
        });
    }

    Ok(bundle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::task::{Task, TaskList};

    #[test]
    fn encode_decode_roundtrip() {
        let list = TaskList {
            id: "l1".into(),
            title: "Work".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        let task = Task {
            id: "t1".into(),
            list_id: "l1".into(),
            title: "Do it".into(),
            done: true,
            link: "".into(),
            comment: "".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        let bundle = TaskBundle::new("0.1.0", vec![list], vec![task]);
        let json = encode_bundle(&bundle).unwrap();
        let back = decode_bundle(&json).unwrap();
        assert_eq!(back.schema_version, SCHEMA_VERSION);
        assert_eq!(back.tasks.len(), 1);
        assert_eq!(back.task_lists.len(), 1);
        assert!(back.tasks[0].done);
    }

    #[test]
    fn rejects_malformed_json() {
        assert!(decode_bundle("not json at all {").is_err());
        assert!(decode_bundle("[1,2,3]").is_err());
        assert!(decode_bundle("").is_err());
    }

    #[test]
    fn rejects_newer_schema_version() {
        let json = r#"{
            "schema_version": 999,
            "tasks": [],
            "task_lists": []
        }"#;
        match decode_bundle(json) {
            Err(SyncError::UnsupportedSchema { found: 999, .. }) => {}
            other => panic!("expected UnsupportedSchema, got {other:?}"),
        }
    }

    #[test]
    fn tolerates_missing_optional_fields() {
        let json = r#"{
            "schema_version": 1,
            "task_lists": [{
                "id": "l1",
                "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-01T00:00:00Z"
            }],
            "tasks": [{
                "id": "x",
                "list_id": "l1",
                "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-01T00:00:00Z"
            }]
        }"#;
        let bundle = decode_bundle(json).unwrap();
        assert_eq!(bundle.task_lists[0].title, "");
        assert_eq!(bundle.tasks[0].title, "");
    }
}
