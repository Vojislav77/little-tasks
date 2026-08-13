// core/task.rs
//
// Task domain types, validation and time helpers.
// This module is intentionally free of any desktop / Tauri / storage
// dependencies so it can be reused in other environments later
// (CLI, mobile, sync service, etc.).

use serde::{Deserialize, Serialize};

pub const LIST_TITLE_MAX_LEN: usize = 100;
pub const TITLE_MAX_LEN: usize = 200;
pub const LINK_MAX_LEN: usize = 2000;
pub const COMMENT_MAX_LEN: usize = 20_000;

/// A fully materialized task list as stored/transported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskList {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

/// A fully materialized task as stored/transported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub list_id: String,
    pub title: String,
    pub done: bool,
    pub link: String,
    pub comment: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Task {
    /// Plain-text preview of a task (uses the comment, then the title).
    #[allow(dead_code)]
    pub fn preview(&self, max_chars: usize) -> String {
        let text = if self.comment.trim().is_empty() {
            self.title.trim().to_string()
        } else {
            self.comment.trim().to_string()
        };
        if text.chars().count() <= max_chars {
            text
        } else {
            let mut out: String = text.chars().take(max_chars).collect();
            out.push_str("…");
            out
        }
    }
}

/// Errors that can come out of validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    EmptyId,
    EmptyTitle,
    TitleTooLong,
    ListTitleTooLong,
    LinkTooLong,
    CommentTooLong,
    BadTimestamp(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::EmptyId => write!(f, "id must not be empty"),
            ValidationError::EmptyTitle => write!(f, "title must not be empty"),
            ValidationError::TitleTooLong => {
                write!(f, "title exceeds {TITLE_MAX_LEN} characters")
            }
            ValidationError::ListTitleTooLong => {
                write!(f, "list title exceeds {LIST_TITLE_MAX_LEN} characters")
            }
            ValidationError::LinkTooLong => write!(f, "link exceeds {LINK_MAX_LEN} characters"),
            ValidationError::CommentTooLong => {
                write!(f, "comment exceeds {COMMENT_MAX_LEN} characters")
            }
            ValidationError::BadTimestamp(ts) => write!(f, "invalid timestamp {ts:?}"),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Validates and normalizes a task list before it hits the storage layer.
pub fn validate_task_list(list: &TaskList) -> Result<TaskList, ValidationError> {
    if list.id.trim().is_empty() {
        return Err(ValidationError::EmptyId);
    }
    if list.title.trim().is_empty() {
        return Err(ValidationError::EmptyTitle);
    }
    if list.title.chars().count() > LIST_TITLE_MAX_LEN {
        return Err(ValidationError::ListTitleTooLong);
    }
    let _ = parse_iso8601(&list.created_at)?;
    let _ = parse_iso8601(&list.updated_at)?;

    Ok(TaskList {
        id: list.id.trim().to_string(),
        title: list.title.trim().to_string(),
        created_at: list.created_at.clone(),
        updated_at: list.updated_at.clone(),
    })
}

/// Validates and normalizes a task before it hits the storage layer.
pub fn validate_task(task: &Task) -> Result<Task, ValidationError> {
    if task.id.trim().is_empty() {
        return Err(ValidationError::EmptyId);
    }
    if task.list_id.trim().is_empty() {
        return Err(ValidationError::EmptyId);
    }
    if task.title.trim().is_empty() {
        return Err(ValidationError::EmptyTitle);
    }
    if task.title.chars().count() > TITLE_MAX_LEN {
        return Err(ValidationError::TitleTooLong);
    }
    if task.link.chars().count() > LINK_MAX_LEN {
        return Err(ValidationError::LinkTooLong);
    }
    if task.comment.chars().count() > COMMENT_MAX_LEN {
        return Err(ValidationError::CommentTooLong);
    }
    let _ = parse_iso8601(&task.created_at)?;
    let _ = parse_iso8601(&task.updated_at)?;

    Ok(Task {
        id: task.id.trim().to_string(),
        list_id: task.list_id.trim().to_string(),
        title: task.title.trim().to_string(),
        done: task.done,
        link: task.link.clone(),
        comment: task.comment.clone(),
        created_at: task.created_at.clone(),
        updated_at: task.updated_at.clone(),
    })
}

/// Minimal ISO8601 parser used for validation and comparison.
/// Accepts e.g. `2026-08-03T18:28:00Z` or with fractional seconds
/// and a numeric offset `+05:00`.
pub fn parse_iso8601(s: &str) -> Result<i64, ValidationError> {
    let err = || ValidationError::BadTimestamp(s.to_string());
    let bytes = s.as_bytes();
    if bytes.len() < 19 {
        return Err(err());
    }
    // YYYY-MM-DDTHH:MM:SS
    let digits_ok = |range: std::ops::Range<usize>| {
        bytes[range.clone()].iter().all(|b| b.is_ascii_digit())
    };
    if !digits_ok(0..4) || !digits_ok(5..7) || !digits_ok(8..10) {
        return Err(err());
    }
    if !digits_ok(11..13) || !digits_ok(14..16) || !digits_ok(17..19) {
        return Err(err());
    }
    let year: i64 = s[0..4].parse().map_err(|_| err())?;
    let month: i64 = s[5..7].parse().map_err(|_| err())?;
    let day: i64 = s[8..10].parse().map_err(|_| err())?;
    let hour: i64 = s[11..13].parse().map_err(|_| err())?;
    let minute: i64 = s[14..16].parse().map_err(|_| err())?;
    let second: i64 = s[17..19].parse().map_err(|_| err())?;

    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return Err(err());
    }

    // epoch seconds from UTC naive components
    let days = days_from_civil(year, month, day)?;
    let naive = days * 86_400 + hour * 3600 + minute * 60 + second;

    // offset handling
    let rest = &s[19..];
    let offset_secs = if rest.is_empty() {
        0
    } else if rest.starts_with('Z') || rest.starts_with('z') {
        0
    } else if rest.starts_with('+') || rest.starts_with('-') {
        // +HH:MM or +HHMM
        let sign = if rest.starts_with('-') { -1 } else { 1 };
        let t = rest[1..].trim_end_matches('Z');
        let t = t.trim_end_matches('z');
        let (h, m) = match t.len() {
            5 if t.as_bytes().get(2) == Some(&b':') => {
                let h: i64 = t[0..2].parse().map_err(|_| err())?;
                let m: i64 = t[3..5].parse().map_err(|_| err())?;
                (h, m)
            }
            4 => {
                let h: i64 = t[0..2].parse().map_err(|_| err())?;
                let m: i64 = t[2..4].parse().map_err(|_| err())?;
                (h, m)
            }
            _ => return Err(err()),
        };
        if h > 23 || m > 59 {
            return Err(err());
        }
        sign * (h * 3600 + m * 60)
    } else {
        return Err(err());
    };

    Ok(naive - offset_secs)
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_from_civil(y: i64, m: i64, d: i64) -> Result<i64, ValidationError> {
    let days_in_month = match m {
        2 => {
            if is_leap_year(y) {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if d > days_in_month {
        return Err(ValidationError::BadTimestamp(format!("{y}-{m}-{d}")));
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Ok(era * 146_097 + doe - 719_468)
}

pub fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    format_iso8601_utc(secs)
}

/// Format epoch seconds as UTC ISO8601 (`YYYY-MM-DDTHH:MM:SSZ`).
pub fn format_iso8601_utc(epoch_secs: i64) -> String {
    let days = epoch_secs.div_euclid(86_400);
    let secs_of_day = epoch_secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    let h = secs_of_day / 3600;
    let mi = (secs_of_day % 3600) / 60;
    let s = secs_of_day % 60;
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn preview_truncates_at_char_boundary() {
        let t = task("1", "l", "x");
        let t = Task {
            comment: "y".repeat(120),
            ..t
        };
        let p = t.preview(80);
        assert_eq!(p.chars().count(), 81);
        assert!(p.ends_with('…'));
    }

    #[test]
    fn preview_uses_comment_when_present() {
        let t = task("1", "l", "title");
        let t = Task {
            comment: "  hello world  ".into(),
            ..t
        };
        assert_eq!(t.preview(80), "hello world");
    }

    #[test]
    fn validation_rejects_empty_title_and_bad_timestamps() {
        let mut l = list("abc", "   ");
        assert_eq!(
            validate_task_list(&l).unwrap_err(),
            ValidationError::EmptyTitle
        );

        l.title = "ok".into();
        l.created_at = "not-a-date".into();
        assert_eq!(
            validate_task_list(&l).unwrap_err(),
            ValidationError::BadTimestamp("not-a-date".into())
        );

        let mut t = task("abc", "l", "task");
        t.created_at = "not-a-date".into();
        assert_eq!(
            validate_task(&t).unwrap_err(),
            ValidationError::BadTimestamp("not-a-date".into())
        );
    }

    #[test]
    fn validation_trims_title_and_rejects_long_fields() {
        let l = list("a", "  Work  ");
        assert_eq!(validate_task_list(&l).unwrap().title, "Work");

        let t = task("b", "a", "T");
        let t = Task {
            link: "x".repeat(5000),
            ..t
        };
        assert_eq!(validate_task(&t).unwrap_err(), ValidationError::LinkTooLong);

        let t = task("c", "a", "T");
        let t = Task {
            comment: "y".repeat(50_000),
            ..t
        };
        assert_eq!(
            validate_task(&t).unwrap_err(),
            ValidationError::CommentTooLong
        );
    }

    #[test]
    fn iso8601_parsing_and_roundtrip() {
        let s = now_iso8601();
        assert!(parse_iso8601(&s).is_ok());
        let with_offset = "2026-08-03T10:00:00+05:30";
        let z = parse_iso8601(with_offset).unwrap();
        let as_utc = format_iso8601_utc(z);
        assert_eq!(as_utc, "2026-08-03T04:30:00Z");
        assert!(parse_iso8601("2026-13-01T00:00:00Z").is_err());
        assert!(parse_iso8601("2026-02-30T00:00:00Z").is_err());
    }
}
