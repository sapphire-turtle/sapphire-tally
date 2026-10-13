//! The two forms — a stamp and an activity — as plain data, and what they
//! turn into.
//!
//! Kept free of egui so the rules are testable. The server checks everything
//! again; these checks only let the window say what is wrong before sending.

use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone as _};
use grain_id::GrainId;
use sapphire_tally_core::{Activity, Stamp, Unit};
use sapphire_tally_mcp::methods::{
    ActivityAddParams, ActivityUpdateParams, StampAddParams, StampUpdateParams,
};

/// How the form shows and reads a stamp's time: minutes, in the stamp's own
/// offset. A full RFC 3339 timestamp is accepted too.
pub const TIME_FORMAT: &str = "%Y-%m-%d %H:%M";

pub const UNITS: [Unit; 3] = [Unit::Day, Unit::Week, Unit::Month];

pub fn unit_name(unit: Unit) -> &'static str {
    match unit {
        Unit::Day => "day",
        Unit::Week => "week",
        Unit::Month => "month",
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StampForm {
    pub activity: Option<GrainId>,
    /// [`TIME_FORMAT`], or RFC 3339.
    pub when: String,
    pub comment: String,
    /// The offset a time without one is read in.
    offset: FixedOffset,
    /// The stamp's time as stored, kept when `when` is left as shown: the
    /// form shows minutes, and saving a comment must not drop the seconds.
    original: Option<DateTime<FixedOffset>>,
}

impl StampForm {
    /// An empty form for a new stamp at `now`.
    pub fn new(activity: Option<GrainId>, now: DateTime<FixedOffset>) -> StampForm {
        StampForm {
            activity,
            when: now.format(TIME_FORMAT).to_string(),
            comment: String::new(),
            offset: *now.offset(),
            original: None,
        }
    }

    /// A form holding `stamp`, to change it.
    pub fn edit(stamp: &Stamp) -> StampForm {
        StampForm {
            activity: Some(stamp.activity),
            when: stamp.timestamp.format(TIME_FORMAT).to_string(),
            comment: stamp.comment.clone().unwrap_or_default(),
            offset: *stamp.timestamp.offset(),
            original: Some(stamp.timestamp),
        }
    }

    pub fn timestamp(&self) -> Result<DateTime<FixedOffset>, String> {
        let text = self.when.trim();
        if let Some(original) = self.original
            && text == original.format(TIME_FORMAT).to_string()
        {
            return Ok(original);
        }
        if let Ok(ts) = DateTime::parse_from_rfc3339(text) {
            return Ok(ts);
        }
        let naive = NaiveDateTime::parse_from_str(text, TIME_FORMAT)
            .or_else(|_| NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S"))
            .map_err(|_| format!("the time must look like 2026-09-05 09:12, got `{text}`"))?;
        self.offset
            .from_local_datetime(&naive)
            .single()
            .ok_or_else(|| format!("`{text}` is not a time in {}", self.offset))
    }

    fn activity(&self) -> Result<GrainId, String> {
        self.activity
            .ok_or_else(|| "choose the activity this stamp is for".to_owned())
    }

    pub fn to_add_params(&self) -> Result<StampAddParams, String> {
        Ok(StampAddParams {
            activity: self.activity()?.to_string(),
            timestamp: Some(self.timestamp()?.to_rfc3339()),
            comment: Some(self.comment.clone()),
        })
    }

    /// Every field, so an emptied comment is removed.
    pub fn to_update_params(&self, id: GrainId) -> Result<StampUpdateParams, String> {
        Ok(StampUpdateParams {
            id: id.to_string(),
            activity: Some(self.activity()?.to_string()),
            timestamp: Some(self.timestamp()?.to_rfc3339()),
            comment: Some(self.comment.clone()),
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActivityForm {
    pub title: String,
    pub unit: Unit,
    pub description: String,
}

impl ActivityForm {
    /// A form holding `activity`, to change it.
    pub fn edit(activity: &Activity) -> ActivityForm {
        ActivityForm {
            title: activity.title.clone(),
            unit: activity.unit.unwrap_or_default(),
            description: activity.description.clone().unwrap_or_default(),
        }
    }

    fn title(&self) -> Result<String, String> {
        let title = self.title.trim();
        if title.is_empty() {
            return Err("an activity needs a title".to_owned());
        }
        Ok(title.to_owned())
    }

    pub fn to_add_params(&self) -> Result<ActivityAddParams, String> {
        Ok(ActivityAddParams {
            title: self.title()?,
            unit: Some(unit_name(self.unit).to_owned()),
            description: Some(self.description.clone()),
        })
    }

    /// Every field, so an emptied description is removed.
    pub fn to_update_params(&self, id: GrainId) -> Result<ActivityUpdateParams, String> {
        Ok(ActivityUpdateParams {
            id: id.to_string(),
            title: Some(self.title()?),
            unit: Some(unit_name(self.unit).to_owned()),
            description: Some(self.description.clone()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(s: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(s).unwrap()
    }

    fn id(s: &str) -> GrainId {
        s.parse().unwrap()
    }

    #[test]
    fn a_new_stamp_reads_its_time_in_the_local_offset() {
        let mut form = StampForm::new(Some(id("012atvw")), ts("2026-09-05T09:12:34+09:00"));
        assert_eq!(form.when, "2026-09-05 09:12");
        form.when = "2026-09-04 21:00".into();
        let p = form.to_add_params().unwrap();
        assert_eq!(p.timestamp.as_deref(), Some("2026-09-04T21:00:00+09:00"));
        assert_eq!(p.activity, "012atvw");
    }

    #[test]
    fn an_unchanged_time_keeps_its_seconds() {
        let stamp = Stamp {
            id: id("012atvx"),
            activity: id("012atvw"),
            timestamp: ts("2026-09-05T09:12:34.5+02:00"),
            comment: Some("old".into()),
        };
        let mut form = StampForm::edit(&stamp);
        form.comment.clear();
        let p = form.to_update_params(stamp.id).unwrap();
        assert_eq!(p.timestamp, Some(stamp.timestamp.to_rfc3339()));
        assert_eq!(p.comment.as_deref(), Some(""));

        // A changed time is read in the stamp's own offset.
        form.when = "2026-09-05 10:00".into();
        assert_eq!(form.timestamp().unwrap(), ts("2026-09-05T10:00:00+02:00"));
    }

    #[test]
    fn a_bad_time_or_no_activity_is_said_before_sending() {
        let mut form = StampForm::new(None, ts("2026-09-05T09:12:00+09:00"));
        assert!(form.to_add_params().unwrap_err().contains("activity"));
        form.activity = Some(id("012atvw"));
        form.when = "yesterday".into();
        assert!(form.to_add_params().unwrap_err().contains("yesterday"));
        form.when = "2026-09-05T01:02:03Z".into();
        assert!(form.to_add_params().is_ok());
    }

    #[test]
    fn an_activity_needs_a_title() {
        let mut form = ActivityForm {
            title: "  ".into(),
            ..ActivityForm::default()
        };
        assert!(form.to_add_params().is_err());
        form.title = " 筋トレ ".into();
        form.unit = Unit::Week;
        let p = form.to_add_params().unwrap();
        assert_eq!(p.title, "筋トレ");
        assert_eq!(p.unit.as_deref(), Some("week"));
    }
}
