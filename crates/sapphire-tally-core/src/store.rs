//! Reading and writing records as Markdown with YAML front matter.
//!
//! ```text
//! ---
//! id: 012atvx
//! activity: 012atvw
//! timestamp: "2026-09-05T09:12:00+09:00"
//! ---
//!
//! Bought it in Yokohama.
//! ```
//!
//! The free-text field (an activity's `description`, a stamp's `comment`) is
//! the body; everything else is front matter.

use std::path::{Path, PathBuf};

use chrono::{DateTime, FixedOffset};
use grain_id::GrainId;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::model::{Activity, Stamp, Unit};

/// The extension every record file has.
pub const EXTENSION: &str = "md";

pub fn activity_path(root: &Path, id: GrainId) -> PathBuf {
    root.join("activities").join(format!("{id}.{EXTENSION}"))
}

pub fn stamp_path(root: &Path, id: GrainId) -> PathBuf {
    root.join("stamps").join(format!("{id}.{EXTENSION}"))
}

#[derive(Serialize, Deserialize)]
struct ActivityFront {
    id: GrainId,
    title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    unit: Option<Unit>,
}

#[derive(Serialize, Deserialize)]
struct StampFront {
    id: GrainId,
    activity: GrainId,
    timestamp: DateTime<FixedOffset>,
}

pub fn read_activity(root: &Path, id: GrainId) -> Result<Activity> {
    let path = activity_path(root, id);
    let (front, body): (ActivityFront, _) = read(&path)?;
    if front.id != id {
        tracing::warn!(
            path = %path.display(),
            file_id = %front.id,
            filename_id = %id,
            "activity id mismatch; using filename id"
        );
    }
    Ok(Activity {
        id,
        title: front.title,
        unit: front.unit,
        description: body,
    })
}

pub fn read_stamp(root: &Path, id: GrainId) -> Result<Stamp> {
    let path = stamp_path(root, id);
    let (front, body): (StampFront, _) = read(&path)?;
    if front.id != id {
        tracing::warn!(
            path = %path.display(),
            file_id = %front.id,
            filename_id = %id,
            "stamp id mismatch; using filename id"
        );
    }
    Ok(Stamp {
        id,
        activity: front.activity,
        timestamp: front.timestamp,
        comment: body,
    })
}

/// Write a new activity. An existing file is [`Error::AlreadyExists`].
pub fn create_activity(root: &Path, a: &Activity) -> Result<()> {
    create(&activity_path(root, a.id), &activity_text(a)?)
}

/// Write a new stamp. An existing file is [`Error::AlreadyExists`].
pub fn create_stamp(root: &Path, s: &Stamp) -> Result<()> {
    create(&stamp_path(root, s.id), &stamp_text(s)?)
}

/// Replace an existing activity's file.
pub fn replace_activity(root: &Path, a: &Activity) -> Result<()> {
    std::fs::write(activity_path(root, a.id), activity_text(a)?)?;
    Ok(())
}

/// Replace an existing stamp's file.
pub fn replace_stamp(root: &Path, s: &Stamp) -> Result<()> {
    std::fs::write(stamp_path(root, s.id), stamp_text(s)?)?;
    Ok(())
}

/// Remove a record's file.
pub fn remove(path: &Path) -> Result<()> {
    std::fs::remove_file(path)?;
    Ok(())
}

fn activity_text(a: &Activity) -> Result<String> {
    let front = ActivityFront {
        id: a.id,
        title: a.title.clone(),
        unit: a.unit,
    };
    render(&front, a.description.as_deref())
}

fn stamp_text(s: &Stamp) -> Result<String> {
    let front = StampFront {
        id: s.id,
        activity: s.activity,
        timestamp: s.timestamp,
    };
    render(&front, s.comment.as_deref())
}

fn create(path: &Path, text: &str) -> Result<()> {
    if path.exists() {
        return Err(Error::AlreadyExists(path.display().to_string()));
    }
    std::fs::write(path, text)?;
    Ok(())
}

/// Read a file's front matter as `T`, and its body: `None` when blank.
fn read<T: serde::de::DeserializeOwned>(path: &Path) -> Result<(T, Option<String>)> {
    let raw = std::fs::read_to_string(path)?;
    let (front, body) = sapphire_frontmatter::parse::<T>(&raw).map_err(|e| Error::Parse {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;
    Ok((front, normalize_text(body)))
}

fn render<T: Serialize>(front: &T, body: Option<&str>) -> Result<String> {
    let mut body = body.unwrap_or_default().trim().to_owned();
    if !body.is_empty() {
        body.push('\n');
    }
    sapphire_frontmatter::render(front, &body).map_err(|e| Error::Render(e.to_string()))
}

/// Trim free text, and read blank text as none.
pub fn normalize_text(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(s: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(s).unwrap()
    }

    #[test]
    fn a_stamp_is_front_matter_and_its_comment_is_the_body() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("stamps")).unwrap();
        let stamp = Stamp {
            // All digits: YAML would read it as a number if it were not quoted.
            id: "0000001".parse().unwrap(),
            activity: "012atvw".parse().unwrap(),
            timestamp: ts("2026-09-05T09:12:00+09:00"),
            comment: Some("横浜で買った\n\n- 甘さ控えめ".into()),
        };
        create_stamp(dir.path(), &stamp).unwrap();

        let text = std::fs::read_to_string(stamp_path(dir.path(), stamp.id)).unwrap();
        let (front, body) = sapphire_frontmatter::split(&text).unwrap();
        assert!(!front.contains("comment"), "{text}");
        assert!(front.contains("2026-09-05T09:12:00+09:00"), "{text}");
        assert_eq!(body, "\n横浜で買った\n\n- 甘さ控えめ\n");

        assert_eq!(read_stamp(dir.path(), stamp.id).unwrap(), stamp);
    }

    #[test]
    fn an_activity_without_description_has_no_body() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("activities")).unwrap();
        let activity = Activity {
            id: "012atvw".parse().unwrap(),
            title: "タピオカミルクティー".into(),
            unit: Some(Unit::Week),
            description: None,
        };
        create_activity(dir.path(), &activity).unwrap();

        let text = std::fs::read_to_string(activity_path(dir.path(), activity.id)).unwrap();
        assert!(text.ends_with("---\n"), "{text}");
        assert_eq!(read_activity(dir.path(), activity.id).unwrap(), activity);
    }

    #[test]
    fn a_hand_written_file_reads() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("activities")).unwrap();
        let id: GrainId = "012atvw".parse().unwrap();
        std::fs::write(
            activity_path(dir.path(), id),
            "---\nid: 012atvw\ntitle: 筋トレ\n---\n\n毎朝の腕立て。\n",
        )
        .unwrap();
        let a = read_activity(dir.path(), id).unwrap();
        assert_eq!(a.title, "筋トレ");
        assert_eq!(a.unit, None);
        assert_eq!(a.description.as_deref(), Some("毎朝の腕立て。"));
    }

    #[test]
    fn creating_over_an_existing_file_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("activities")).unwrap();
        let activity = Activity {
            id: "012atvw".parse().unwrap(),
            title: "x".into(),
            unit: None,
            description: None,
        };
        create_activity(dir.path(), &activity).unwrap();
        let err = create_activity(dir.path(), &activity).unwrap_err();
        assert!(matches!(err, Error::AlreadyExists(_)), "{err}");
    }
}
