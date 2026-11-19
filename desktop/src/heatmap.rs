//! The heatmap's arithmetic: which days the grid covers, how many stamps
//! fall on each, and how dark each day is drawn.
//!
//! The grid is GitHub's contribution graph turned to ISO weeks: one column per
//! week, Monday at the top. A day's shade is its count against the busiest day
//! in view, in [`LEVELS`] steps.

use std::collections::BTreeMap;

use grain_id::GrainId;
use jiff::ToSpan;
use jiff::civil::Date;
use sapphire_tally_core::Stamp;

/// How many weeks the grid shows: a year, like GitHub's.
pub const WEEKS: u64 = 53;

/// Shades for a day with stamps; a day without any is level 0.
pub const LEVELS: u8 = 4;

/// The days a grid ending on `end` covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grid {
    /// The Monday of the first column.
    pub start: Date,
    /// The last day shown: the grid stops there, mid-column.
    pub end: Date,
}

impl Grid {
    /// [`WEEKS`] columns, the last one holding `end`.
    pub fn ending(end: Date) -> Grid {
        let monday = end - i64::from(end.weekday().to_monday_zero_offset()).days();
        Grid {
            start: monday - ((WEEKS as i64 - 1) * 7).days(),
            end,
        }
    }

    /// The day at `week` (column) and `weekday` (row, Monday = 0), or `None`
    /// past [`Grid::end`].
    pub fn day(&self, week: u64, weekday: u64) -> Option<Date> {
        let day = self.start + ((week * 7 + weekday) as i64).days();
        (day <= self.end).then_some(day)
    }
}

/// Stamps per day (in this host's time zone, as the server counts them),
/// optionally for one activity.
pub fn day_counts(stamps: &[Stamp], activity: Option<GrainId>) -> BTreeMap<Date, u32> {
    let mut counts = BTreeMap::new();
    for stamp in stamps {
        if activity.is_some_and(|a| a != stamp.activity) {
            continue;
        }
        *counts
            .entry(sapphire_tally_core::time::local_date(stamp.timestamp))
            .or_insert(0) += 1;
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
    use super::*;

    fn date(s: &str) -> Date {
        s.parse().unwrap()
    }

    #[test]
    fn the_grid_starts_on_a_monday_a_year_back() {
        // 2026-10-13 is a Tuesday.
        let grid = Grid::ending(date("2026-10-13"));
        assert_eq!(grid.start.weekday(), jiff::civil::Weekday::Monday);
        assert_eq!(grid.start, date("2025-10-13"));
        assert_eq!(grid.day(52, 0), Some(date("2026-10-12")));
        assert_eq!(grid.day(52, 1), Some(date("2026-10-13")));
        assert_eq!(grid.day(52, 2), None, "the future is not drawn");
    }

    #[test]
    fn days_count_in_the_local_time_zone() {
        let stamp = |activity: &str, ts: &str| Stamp {
            id: GrainId::random(),
            activity: activity.parse().unwrap(),
            timestamp: sapphire_tally_core::time::parse(ts).unwrap(),
            comment: None,
        };
        let stamps = [
            stamp("012atvw", "2026-09-05T23:30:00"),
            stamp("012atvw", "2026-09-05T08:00:00"),
            stamp("012atvx", "2026-09-05T12:00:00"),
            stamp("012atvw", "2026-09-06T00:10:00"),
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
