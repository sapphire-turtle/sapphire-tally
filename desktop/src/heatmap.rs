//! The heatmap's arithmetic: which days the grid covers, how many stamps
//! fall on each, and how dark each day is drawn.
//!
//! The grid is GitHub's contribution graph turned to ISO weeks: one column per
//! week, Monday at the top. A day's shade is its count against the busiest day
//! in view, in [`LEVELS`] steps.

use std::collections::BTreeMap;

use chrono::{Datelike as _, Days, NaiveDate};
use grain_id::GrainId;
use sapphire_tally_core::Stamp;

/// How many weeks the grid shows: a year, like GitHub's.
pub const WEEKS: u64 = 53;

/// Shades for a day with stamps; a day without any is level 0.
pub const LEVELS: u8 = 4;

/// The days a grid ending on `end` covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grid {
    /// The Monday of the first column.
    pub start: NaiveDate,
    /// The last day shown: the grid stops there, mid-column.
    pub end: NaiveDate,
}

impl Grid {
    /// [`WEEKS`] columns, the last one holding `end`.
    pub fn ending(end: NaiveDate) -> Grid {
        let monday = end - Days::new(u64::from(end.weekday().num_days_from_monday()));
        Grid {
            start: monday - Days::new((WEEKS - 1) * 7),
            end,
        }
    }

    /// The day at `week` (column) and `weekday` (row, Monday = 0), or `None`
    /// past [`Grid::end`].
    pub fn day(&self, week: u64, weekday: u64) -> Option<NaiveDate> {
        let day = self.start + Days::new(week * 7 + weekday);
        (day <= self.end).then_some(day)
    }
}

/// Stamps per day (each in its own offset, as the server counts them),
/// optionally for one activity.
pub fn day_counts(stamps: &[Stamp], activity: Option<GrainId>) -> BTreeMap<NaiveDate, u32> {
    let mut counts = BTreeMap::new();
    for stamp in stamps {
        if activity.is_some_and(|a| a != stamp.activity) {
            continue;
        }
        *counts.entry(stamp.timestamp.date_naive()).or_insert(0) += 1;
    }
    counts
}

/// The shade of a day with `count` stamps when the busiest has `max`.
pub fn level(count: u32, max: u32) -> u8 {
    if count == 0 || max == 0 {
        return 0;
    }
    let scaled = (u64::from(count) * u64::from(LEVELS)).div_ceil(u64::from(max));
    scaled.clamp(1, u64::from(LEVELS)) as u8
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use super::*;

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn the_grid_starts_on_a_monday_a_year_back() {
        // 2026-10-13 is a Tuesday.
        let grid = Grid::ending(date("2026-10-13"));
        assert_eq!(grid.start.weekday(), chrono::Weekday::Mon);
        assert_eq!(grid.start, date("2025-10-13"));
        assert_eq!(grid.day(52, 0), Some(date("2026-10-12")));
        assert_eq!(grid.day(52, 1), Some(date("2026-10-13")));
        assert_eq!(grid.day(52, 2), None, "the future is not drawn");
    }

    #[test]
    fn days_count_in_the_stamps_own_offset() {
        let stamp = |activity: &str, ts: &str| Stamp {
            id: GrainId::random(),
            activity: activity.parse().unwrap(),
            timestamp: DateTime::parse_from_rfc3339(ts).unwrap(),
            comment: None,
        };
        let stamps = [
            stamp("012atvw", "2026-09-05T23:30:00+09:00"),
            stamp("012atvw", "2026-09-05T08:00:00+09:00"),
            stamp("012atvx", "2026-09-05T12:00:00+09:00"),
            stamp("012atvw", "2026-09-06T00:10:00+09:00"),
        ];
        let all = day_counts(&stamps, None);
        assert_eq!(all[&date("2026-09-05")], 3);
        assert_eq!(all[&date("2026-09-06")], 1);
        let one = day_counts(&stamps, Some("012atvx".parse().unwrap()));
        assert_eq!(one.len(), 1);
    }

    #[test]
    fn shades_scale_to_the_busiest_day() {
        assert_eq!(level(0, 10), 0);
        assert_eq!(level(1, 10), 1);
        assert_eq!(level(5, 10), 2);
        assert_eq!(level(10, 10), 4);
        assert_eq!(level(1, 1), 4);
        assert_eq!(level(3, 0), 0);
    }
}
