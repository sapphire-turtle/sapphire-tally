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

/// Which way the user wants an activity's count to move. Track (only
/// counted) by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Goal {
    /// More is better: a habit to build.
    Increase,
    /// Fewer is better: a habit to cut down.
    Decrease,
    /// Only counted.
    #[default]
    Track,
}

impl Goal {
    /// Parse "increase"/"decrease"/"track", ignoring case and surrounding
    /// whitespace. A bad value is an `Err(String)` naming it, passed to a tool
    /// error as is.
    pub fn parse_goal(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "increase" => Ok(Goal::Increase),
            "decrease" => Ok(Goal::Decrease),
            "track" => Ok(Goal::Track),
            other => Err(format!(
                "goal must be one of increase/decrease/track, got `{other}`"
            )),
        }
    }
}

/// A named colour of the theme's: it follows the theme. Mirrors corundum's
/// `theme::Swatch`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Swatch {
    Neutral,
    /// The user's accent colour.
    Accent,
    Red,
    Orange,
    Yellow,
    Green,
    Cyan,
    Blue,
    Purple,
}

impl Swatch {
    pub const ALL: [Swatch; 9] = [
        Swatch::Neutral,
        Swatch::Accent,
        Swatch::Red,
        Swatch::Orange,
        Swatch::Yellow,
        Swatch::Green,
        Swatch::Cyan,
        Swatch::Blue,
        Swatch::Purple,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Swatch::Neutral => "neutral",
            Swatch::Accent => "accent",
            Swatch::Red => "red",
            Swatch::Orange => "orange",
            Swatch::Yellow => "yellow",
            Swatch::Green => "green",
            Swatch::Cyan => "cyan",
            Swatch::Blue => "blue",
            Swatch::Purple => "purple",
        }
    }
}

/// An activity's colour, with no lightness of its own: where it is drawn
/// decides that, so it follows the light / dark theme. Mirrors corundum's
/// `theme::Tint`.
///
/// Written as a swatch name (`green`) or `oklch(C H)`: an OKLCH chroma in
/// `[0, MAX_CHROMA]` and a hue in degrees, `[0, 360)`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Color {
    Swatch(Swatch),
    Oklch { chroma: f32, hue: f32 },
}

impl Color {
    /// The most chroma a colour may have: corundum's `Tint::MAX_CHROMA`.
    pub const MAX_CHROMA: f32 = 0.25;
}

impl std::str::FromStr for Color {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        let bad = || {
            format!(
                "color must be a swatch name \
                 (neutral/accent/red/orange/yellow/green/cyan/blue/purple) or oklch(C H) \
                 with 0 ≤ C ≤ 0.25 and 0 ≤ H < 360, got `{s}`"
            )
        };
        let lower = s.trim().to_ascii_lowercase();
        if let Some(swatch) = Swatch::ALL.into_iter().find(|w| w.name() == lower) {
            return Ok(Color::Swatch(swatch));
        }
        let inner = lower
            .strip_prefix("oklch(")
            .and_then(|r| r.strip_suffix(')'))
            .ok_or_else(bad)?;
        let mut parts = inner.split_whitespace().map(str::parse::<f32>);
        let (Some(Ok(chroma)), Some(Ok(hue)), None) = (parts.next(), parts.next(), parts.next())
        else {
            return Err(bad());
        };
        if !(0.0..=Color::MAX_CHROMA).contains(&chroma) || !(0.0..360.0).contains(&hue) {
            return Err(bad());
        }
        Ok(Color::Oklch { chroma, hue })
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Color::Swatch(s) => f.write_str(s.name()),
            Color::Oklch { chroma, hue } => write!(f, "oklch({chroma} {hue})"),
        }
    }
}

impl TryFrom<String> for Color {
    type Error = String;

    fn try_from(s: String) -> Result<Self, String> {
        s.parse()
    }
}

impl From<Color> for String {
    fn from(c: Color) -> String {
        c.to_string()
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
    /// `None` reads as [`Goal::Track`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal: Option<Goal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
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
    pub goal: Option<Goal>,
    /// `Some(None)` removes the colour.
    pub color: Option<Option<Color>>,
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

    #[test]
    fn invalid_goal_is_rejected_by_parse_goal() {
        let e = Goal::parse_goal("less").unwrap_err();
        assert!(
            e.contains("less") && e.contains("increase/decrease/track"),
            "{e}"
        );
        assert_eq!(Goal::parse_goal(" Decrease ").unwrap(), Goal::Decrease);
    }

    #[test]
    fn color_parses_both_forms_and_roundtrips() {
        let green: Color = " Green ".parse().unwrap();
        assert_eq!(green, Color::Swatch(Swatch::Green));
        let custom: Color = "OKLCH(0.15 255)".parse().unwrap();
        assert_eq!(
            custom,
            Color::Oklch {
                chroma: 0.15,
                hue: 255.0
            }
        );
        for c in [green, custom] {
            assert_eq!(c.to_string().parse::<Color>().unwrap(), c);
        }
        assert_eq!(green.to_string(), "green");
    }

    #[test]
    fn bad_colors_are_rejected() {
        for bad in [
            "oklch(0.3 10)",
            "oklch(0.1 360)",
            "oklch(nan 10)",
            "oklch(0.1)",
            "pink",
        ] {
            let e = bad.parse::<Color>().unwrap_err();
            assert!(e.contains(bad) && e.contains("oklch(C H)"), "{e}");
        }
    }
}
