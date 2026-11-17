# Activity colour on the heatmap

Issue: #54. Short design (a small change). Ships with #53 (`APP_API` 2).

## Why

One heatmap per activity, all in GitHub green, makes activities hard to tell
apart at a glance. Each activity gets its own colour: picked by hand with
corundum's picker, or derived from its title.

## Goals / non-goals

- An activity may store a colour; without one the desktop derives it from the
  title, the same on every device.
- Each heatmap's shades use the activity's colour and follow the
  light / dark theme.
- MCP can set, clear and read the colour. Log tools (`stamp_list`, `heatmap`)
  do not carry it: it does not change what a count means.
- Non-goal: colours in the CLI, colour per stamp.

## Approaches

- **One string, corundum's two kinds (picked).** `color: green` (a swatch
  name) or `color: oklch(0.15 255)` (chroma, hue), the same text in front
  matter and tool arguments. Maps 1:1 to `corundum_framework_gui::theme::Tint`,
  so the colour follows the theme.
- `#rrggbb`: familiar, but `Tint` has no lightness; a hex colour would fight
  the theme's ladder in one of the two modes.
- Store `Tint` itself: needs core to depend on the gui crate. Ruled out.

## Design

- `sapphire-tally-core::model`:
  - `enum Swatch { Neutral, Accent, Red, Orange, Yellow, Green, Cyan, Blue, Purple }`
    (lowercase names), `enum Color { Swatch(Swatch), Oklch { chroma: f32, hue: f32 } }`.
  - `Color: FromStr + Display`, serialized as that string. Parsing ignores case
    and surrounding whitespace; `oklch(C H)` needs finite `0 ≤ C ≤ 0.25` and
    `0 ≤ H < 360`. A bad value is an error naming it and the accepted forms.
  - `Activity.color: Option<Color>` in front matter, skipped when `None`.
    `ActivityUpdate.color: Option<Option<Color>>` (`Some(None)` clears).
- `sapphire-tally-mcp`: `activity_add` / `activity_update` take `color:
  Option<String>`; on update an empty string clears it. Descriptions say so.
  `activity_list` / `activity_get` carry it through `Activity`.
- Desktop:
  - `Color ↔ Tint` conversion (swatches 1:1, `Oklch` ↔ `Tint::Custom`).
  - Unset colour → `Tint::from_bytes(title.as_bytes())`; never written.
  - Activity form: `TintPicker::new(&mut tint, Tint::from_bytes(title))`;
    Default sets it back to unset.
  - The toolbar's legend is shared by every heatmap, so it is drawn in the
    neutral swatch: it shows intensity, not any one activity.
  - Heatmap: level 0 = the theme's neutral at `Tone::Raised`;
    levels 1–4 = the activity's tint at `Tone::Pressed`, `Border`, `Muted`,
    `Text` via `Theme::tint`, from `theme::current(ctx)`.

## Left to the builder

The tone choice for levels 1–4 may be adjusted after looking at it on screen;
names of the conversion helpers.
