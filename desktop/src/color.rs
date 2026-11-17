//! An activity's colour on screen: the core's [`Color`] as corundum's
//! [`Tint`], the colour of an activity that has none, and a heatmap's shades.

use sapphire_framework::gui::egui::Color32;
use sapphire_framework::gui::theme::{Swatch as GuiSwatch, Theme, Tint, Tone};
use sapphire_tally_core::{Color, Swatch};

pub fn to_tint(c: Color) -> Tint {
    match c {
        Color::Swatch(s) => Tint::Swatch(match s {
            Swatch::Neutral => GuiSwatch::Neutral,
            Swatch::Accent => GuiSwatch::Accent,
            Swatch::Red => GuiSwatch::Red,
            Swatch::Orange => GuiSwatch::Orange,
            Swatch::Yellow => GuiSwatch::Yellow,
            Swatch::Green => GuiSwatch::Green,
            Swatch::Cyan => GuiSwatch::Cyan,
            Swatch::Blue => GuiSwatch::Blue,
            Swatch::Purple => GuiSwatch::Purple,
        }),
        Color::Oklch { chroma, hue } => Tint::Custom { chroma, hue },
    }
}

pub fn from_tint(t: Tint) -> Color {
    match t {
        Tint::Swatch(s) => Color::Swatch(match s {
            GuiSwatch::Neutral => Swatch::Neutral,
            GuiSwatch::Accent => Swatch::Accent,
            GuiSwatch::Red => Swatch::Red,
            GuiSwatch::Orange => Swatch::Orange,
            GuiSwatch::Yellow => Swatch::Yellow,
            GuiSwatch::Green => Swatch::Green,
            GuiSwatch::Cyan => Swatch::Cyan,
            GuiSwatch::Blue => Swatch::Blue,
            GuiSwatch::Purple => Swatch::Purple,
        }),
        // The picker's disk keeps a hue within [0, 360) and a chroma within
        // [0, MAX_CHROMA]; the core refuses anything else, so bring it in.
        Tint::Custom { chroma, hue } => Color::Oklch {
            chroma: chroma.clamp(0.0, Color::MAX_CHROMA),
            hue: hue.rem_euclid(360.0) % 360.0,
        },
    }
}

/// The tint an activity is drawn in: its colour, or the one its title hashes
/// to — the same on every device — when it has none. The title is trimmed,
/// as the form saves it, so saving does not change the colour.
pub fn tint_of(color: Option<Color>, title: &str) -> Tint {
    color.map_or_else(|| Tint::from_bytes(title.trim().as_bytes()), to_tint)
}

/// A heatmap's shades in `tint`, level 0 (no stamps) to
/// [`crate::heatmap::LEVELS`]: even steps from an empty cell to the tint at
/// [`Tone::Muted`], like GitHub's ramp. The theme's own tones bunch up
/// (Pressed and Border are close), and the top stays clear of the strong text
/// colour the picked day is outlined in.
pub fn shades(theme: &Theme, tint: Tint) -> [Color32; 5] {
    let empty = theme.color(Tone::Raised, GuiSwatch::Neutral);
    let full = theme.tint(Tone::Muted, tint);
    std::array::from_fn(|level| empty.lerp_to_gamma(full, level as f32 / 4.0))
}

#[cfg(test)]
mod tests {
    use sapphire_framework::gui::theme::Contrast;

    use super::*;

    #[test]
    fn tints_roundtrip() {
        let custom = Color::Oklch {
            chroma: 0.15,
            hue: 255.0,
        };
        for c in Swatch::ALL.map(Color::Swatch).into_iter().chain([custom]) {
            assert_eq!(from_tint(to_tint(c)), c);
        }
        assert_eq!(
            to_tint(Color::Swatch(Swatch::Green)),
            Tint::Swatch(GuiSwatch::Green)
        );
    }

    #[test]
    fn an_unset_colour_comes_from_the_title() {
        assert_eq!(tint_of(None, "筋トレ"), tint_of(None, "筋トレ"));
        assert_ne!(tint_of(None, "筋トレ"), tint_of(None, "読書"));
        assert_eq!(
            tint_of(None, "筋トレ"),
            Tint::from_bytes("筋トレ".as_bytes())
        );
        assert_eq!(
            tint_of(Some(Color::Swatch(Swatch::Green)), "筋トレ"),
            Tint::Swatch(GuiSwatch::Green)
        );
    }

    #[test]
    fn shades_are_distinct_in_both_themes() {
        for dark in [false, true] {
            let theme = Theme::new(dark, Contrast::Medium, Tint::Swatch(GuiSwatch::Blue));
            let s = shades(&theme, Tint::Swatch(GuiSwatch::Green));
            for i in 0..s.len() {
                for j in i + 1..s.len() {
                    assert_ne!(s[i], s[j], "dark={dark} levels {i} and {j}");
                }
            }
        }
    }

    #[test]
    fn a_titles_surrounding_spaces_do_not_change_its_colour() {
        assert_eq!(tint_of(None, " 筋トレ "), tint_of(None, "筋トレ"));
    }

    /// Each step of the ramp is visible: GitHub's greens are 30+ apart.
    #[test]
    fn every_step_of_the_ramp_is_visible() {
        let dist = |a: Color32, b: Color32| {
            let d = |x: u8, y: u8| (f32::from(x) - f32::from(y)).powi(2);
            (d(a.r(), b.r()) + d(a.g(), b.g()) + d(a.b(), b.b())).sqrt()
        };
        for dark in [false, true] {
            for contrast in [Contrast::Low, Contrast::Medium, Contrast::High] {
                let theme = Theme::new(dark, contrast, Tint::Swatch(GuiSwatch::Blue));
                for tint in [
                    Tint::Swatch(GuiSwatch::Green),
                    Tint::Swatch(GuiSwatch::Neutral),
                    Tint::from_bytes(b"x"),
                ] {
                    let s = shades(&theme, tint);
                    for i in 0..s.len() - 1 {
                        let step = dist(s[i], s[i + 1]);
                        assert!(
                            step >= 20.0,
                            "dark={dark} {contrast:?} {tint:?}: levels {i}->{} only {step}",
                            i + 1
                        );
                    }
                }
            }
        }
    }
}
