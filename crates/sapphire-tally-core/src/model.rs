use chrono::{DateTime, FixedOffset};
use grain_id::GrainId;
use serde::{Deserialize, Serialize};

/// Heatmap granularity, set per activity. Day by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
    #[default]
    Day,
    Week,
    Month,
}

impl Unit {
    /// Parse "day"/"week"/"month", ignoring case and surrounding whitespace.
    /// A bad value is an `Err(String)` naming it, passed to a tool error as is.
    pub fn parse_unit(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "day" => Ok(Unit::Day),
            "week" => Ok(Unit::Week),
            "month" => Ok(Unit::Month),
            other => Err(format!("unit must be one of day/week/month, got `{other}`")),
        }
    }
}

/// An activity (tracker): `activities/<id>.md`.
///
/// Every field but `description` is front matter; `description` is the
/// Markdown body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Activity {
    /// Repeats the filename so the file stands on its own. The filename wins
    /// when they disagree, with a warning.
    pub id: GrainId,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<Unit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A stamp: `stamps/<id>.md`, one file each.
///
/// Every field but `comment` is front matter; `comment` is the Markdown body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stamp {
    /// Repeats the filename so the file stands on its own. The filename wins
    /// when they disagree, with a warning.
    pub id: GrainId,
    pub activity: GrainId,
    pub timestamp: DateTime<FixedOffset>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

/// A change to an activity: `None` keeps a field as it is. An empty
/// `description` removes it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ActivityUpdate {
    pub title: Option<String>,
    pub unit: Option<Unit>,
    pub description: Option<String>,
}

/// A change to a stamp: `None` keeps a field as it is. An empty `comment`
/// removes it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StampUpdate {
    pub activity: Option<GrainId>,
    pub timestamp: Option<DateTime<FixedOffset>>,
    pub comment: Option<String>,
}

/// One heatmap cell. `period` is "2026-09-05" / "2026-W36" / "2026-09".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatmapBucket {
    pub activity: GrainId,
    pub period: String,
    pub count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_unit_is_rejected_by_parse_unit() {
        let e = Unit::parse_unit("fortnight").unwrap_err();
        assert!(e.contains("fortnight"), "{e}");
        assert_eq!(Unit::parse_unit("WEEK").unwrap(), Unit::Week);
    }
}
