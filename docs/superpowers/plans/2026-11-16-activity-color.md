# Activity Colour Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Each activity may carry a colour (`green` or `oklch(C H)`); the desktop heatmap draws each activity in its colour, or one derived from its title.

**Architecture:** `Color` lives in `sapphire-tally-core` as a string-serialized value mirroring corundum's `Tint`. MCP sets/clears it on add/update. The desktop converts it to `Tint`, picks it with `TintPicker`, and shades the heatmap with `Theme::tint`.

**Tech Stack:** Rust 2024, serde, rmcp, egui + `sapphire_framework::gui::theme` (corundum-framework-gui 0.11).

**Spec:** `docs/superpowers/specs/2026-11-16-activity-color-design.md` (issue #54)

## Global Constraints

- Text forms are exactly: one of `neutral accent red orange yellow green cyan blue purple`, or `oklch(C H)` with finite `0 ≤ C ≤ 0.25`, `0 ≤ H < 360`. Parsing ignores case and surrounding whitespace. Display writes the lowercase name or `oklch({chroma} {hue})` with Rust's `{}` float formatting.
- Front matter omits `color` when `None`; unset colour is never written by the desktop.
- MCP `activity_update`: omitted `color` keeps it, `""` clears it.
- `APP_API` stays `2` (already bumped by #53 on this branch).
- Run `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` before each commit.

## Review Focus

1. `oklch(0.3 10)` (chroma over 0.25), `oklch(0.1 360)`, `oklch(nan 10)`, `pink` → error naming the value and the accepted forms. (Task 1 test)
2. `" Green "` / `"OKLCH(0.15 255)"` parse; Display → parse round-trips. (Task 1 test)
3. `activity_update` with `color: ""` removes the key from the file; with `color` omitted keeps it. (Task 2 test)
4. A title-derived colour is identical for the same title and is not written to the file when the form is saved. (Task 3 test)
5. Light and dark palettes: level 0 differs from level 1, and levels 1–4 are pairwise distinct. (Task 3 test)

---

### Task 1: `Color` in core

**Files:**
- Modify: `crates/sapphire-tally-core/src/model.rs`, `store.rs` (`ActivityFront.color`), `tally.rs` (`create_activity`, `update_activity`, call sites), `lib.rs` (re-export `Color`, `Swatch`)

**Interfaces:**
- Produces:
  - `pub enum Swatch { Neutral, Accent, Red, Orange, Yellow, Green, Cyan, Blue, Purple }` (`Debug, Clone, Copy, PartialEq, Eq`), `Swatch::ALL: [Swatch; 9]`, `Swatch::name(self) -> &'static str`.
  - `pub enum Color { Swatch(Swatch), Oklch { chroma: f32, hue: f32 } }` (`Debug, Clone, Copy, PartialEq`), `impl FromStr for Color { type Err = String }`, `impl Display`, `Serialize`/`Deserialize` via the string (`#[serde(try_from = "String", into = "String")]`). Error text: ``color must be a swatch name (neutral/accent/red/orange/yellow/green/cyan/blue/purple) or oklch(C H) with 0 ≤ C ≤ 0.25 and 0 ≤ H < 360, got `{s}` ``.
  - `Activity.color: Option<Color>` (skip when `None`), after `goal`. `ActivityUpdate.color: Option<Option<Color>>`.
  - `Tally::create_activity(&self, title, unit, goal, color: Option<Color>, description)`.

- [ ] **Step 1: failing tests** — `model.rs`: `color_parses_both_forms_and_roundtrips` (`" Green "`→`Swatch(Green)`, `"OKLCH(0.15 255)"`→`Oklch{0.15,255.0}`, `c.to_string().parse() == c` for both), `bad_colors_are_rejected` (each of `oklch(0.3 10)`, `oklch(0.1 360)`, `oklch(nan 10)`, `oklch(0.1)`, `pink`: error contains the input). `tally.rs`: `color_is_set_kept_and_cleared` (create with `Some(green)`; update title only → still green; update `color: Some(None)` → `None` and file text has no `color`).
- [ ] **Step 2:** `cargo test -p sapphire-tally-core` → compile errors.
- [ ] **Step 3:** implement; add `None` at existing call sites / literals.
- [ ] **Step 4:** `cargo test -p sapphire-tally-core` → PASS.
- [ ] **Step 5:** commit `feat(core): an activity's colour, a swatch or oklch(C H) (#54)`.

### Task 2: Colour over MCP

**Files:** `crates/sapphire-tally-mcp/src/methods.rs`, `tools.rs`; `crates/sapphire-tally-server/tests/mcp_tools.rs`; `desktop/src/form.rs`, `desktop/src/tally.rs` (`color: None` to compile).

**Interfaces:**
- Consumes: `Color`, `create_activity(.., color, ..)`, `ActivityUpdate.color`.
- Produces: `ActivityAddParams.color: Option<String>`, `ActivityUpdateParams.color: Option<String>` (`#[serde(default)]`, doc: `/// A swatch name (green, blue, …) or oklch(C H). On update an empty string removes it.`).

- [ ] **Step 1: failing tests** in `methods.rs`: `color_is_set_and_cleared_over_mcp` (add with `"green"` → JSON `color == "green"`; update `color: Some("")` → `activity_get` has no `color` key; update other field with `color: None` after setting `"oklch(0.15 255)"` keeps it), `invalid_color_returns_tool_error` (`"pink"` → err contains `pink`).
- [ ] **Step 2:** `cargo test -p sapphire-tally-mcp` → compile errors.
- [ ] **Step 3:** `parse_color(Option<&str>) -> anyhow::Result<Option<Color>>` for add; for update map `""`→`Some(None)`, other→`Some(Some(parsed))`. Tool descriptions for `activity_add`/`activity_update`/`activity_get`/`activity_list` mention `color`.
- [ ] **Step 4:** `cargo test --workspace` + clippy → PASS.
- [ ] **Step 5:** commit `feat(mcp): set and clear an activity's colour (#54)`.

### Task 3: Desktop colours

**Files:** Create `desktop/src/color.rs` (conversion + palette, egui-free besides `Color32`/theme types); modify `desktop/src/main.rs` (`mod color;`), `desktop/src/form.rs`, `desktop/src/tally.rs` (form row, `heatmap_ui`, `legend`, remove old `palette`).

**Interfaces:**
- Consumes: `Color`, `Swatch` (core), `ActivityAddParams.color` / `ActivityUpdateParams.color`; `sapphire_framework::gui::theme::{Theme, Tint, TintPicker, Tone, Swatch as GuiSwatch, Contrast}`.
- Produces:
  - `pub fn to_tint(c: Color) -> Tint`, `pub fn from_tint(t: Tint) -> Color`.
  - `pub fn tint_of(color: Option<Color>, title: &str) -> Tint` (`None` → `Tint::from_bytes(title.as_bytes())`).
  - `pub fn shades(theme: &Theme, tint: Tint) -> [Color32; 5]`: `[theme.color(Tone::Raised, GuiSwatch::Neutral), theme.tint(Pressed|Border|Muted|Text, tint)]`.
  - `ActivityForm.color: Option<Color>`; add params send `color.map(to_string)`; update params send `Some(color.map(to_string).unwrap_or_default())`.
  - `heatmap_ui(ui, grid, counts, picked, tint: Tint)`; legend draws `shades(theme, Tint::Swatch(GuiSwatch::Neutral))`.

- [ ] **Step 1: failing tests** — `color.rs`: `tints_roundtrip` (every core swatch and an `Oklch` → `from_tint(to_tint(c)) == c`), `an_unset_colour_comes_from_the_title` (`tint_of(None,"筋トレ") == tint_of(None,"筋トレ")`, `!= tint_of(None,"読書")`, `tint_of(Some(green), _) == Tint::Swatch(Green)`), `shades_are_distinct_in_both_themes` (for `dark` in both, `Theme::new(dark, Contrast::Medium, Tint::Swatch(Blue))`, the 5 shades of `Tint::Swatch(Green)` pairwise distinct). `form.rs`: `activity_form_sends_its_colour` (unset → add `None`, update `Some("")`; set green → both `Some("green")`).
- [ ] **Step 2:** `cargo test -p sapphire-tally-desktop` → compile errors.
- [ ] **Step 3:** implement; form row "Color" when editing shows `TintPicker::new(&mut tint, Tint::from_bytes(title)).tone(Tone::Muted).show(ui)` and copies `tint` back into `form.color` every frame; when not editing, a filled chip in the tint. Heatmap per activity passes `tint_of(activity.color, &activity.title)`.
- [ ] **Step 4:** `cargo test --workspace` + clippy → PASS.
- [ ] **Step 5:** commit `feat(desktop): each activity's heatmap in its own colour (#54)`.
