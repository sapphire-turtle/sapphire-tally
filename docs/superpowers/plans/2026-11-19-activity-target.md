# Activity Target Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `goal` (#53, unreleased) with `target`: an activity says how many times, in its own `unit`, and whether that is a floor (`above`) or a ceiling (`below`); every MCP log result carries the target and, where there is a count, `status` — how that count stands against it.

**Architecture:** `Bound`, `Target` and `Status` live in `sapphire-tally-core` beside `Unit` and are stored in the activity's front matter as a mapping. `sapphire-tally-mcp` takes the text form (`"3-below"`) on add/update and decorates log entries via the flattened `StampEntry` / `BucketEntry` structs #53 introduced. The desktop activity form's Goal row becomes a Target row.

**Tech Stack:** Rust edition 2024, serde (YAML front matter, JSON), rmcp + schemars, egui (desktop).

**Spec:** `docs/superpowers/specs/2026-11-19-activity-target-design.md` (issue #56)

## Global Constraints

- The front matter / JSON shape is exactly `target:` mapping to `times: <u32>` and `bound: above | below`; the text form (tool arg, `Display`) is `"{times}-{bound}"` — `3-below`.
- Parsing ignores case and surrounding whitespace, like `Unit::parse_unit` / `Color`'s `FromStr`.
- An activity without a target is only counted (数えるだけ): `target` is `skip_serializing_if = "Option::is_none"`, so such a file is unchanged byte for byte.
- `goal` is gone everywhere: `model.rs`, `store.rs`, `tally.rs`, `methods.rs`, `tools.rs`, `desktop/src/form.rs`, `desktop/src/tally.rs`, both READMEs.
- Log entries: `StampEntry` carries `target` (absent when the activity has none) and no `status`; `BucketEntry` carries `target` and `status` (`met` / `under` / `exceeded`), the latter from `Target::status(count)`.
- `APP_API` goes `2` → `3`.
- Run `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` before each commit.

## Review Focus

1. `"3-below"` parses to `Target { times: 3, bound: Below }` and `Display` writes it back (Task 1).
2. A bad target string names the got value and shows the shape (`N-bound`, as in `1-above`) (Task 1 + Task 2).
3. An activity with no target writes no `target` key and its log entries carry no `target` and no `status` (Task 1 + Task 2).
4. `status`: `above`+`count < times` → `under`; `above`+`count >= times` → `met`; `below`+`count > times` → `exceeded`; `below`+`count <= times` → `met`. Read against the bucket's own count (Task 2).
5. `activity_update` without `target` keeps the existing one; an empty `target` removes it (Task 1).
6. An unreadable activity's stamps still count, under its id as title and with no `target` (1cd8f9e kept).

---

### Task 1: `Bound`, `Target` and `Status` in core

**Files:**
- Modify: `crates/sapphire-tally-core/src/model.rs` (replace `Goal` and `Activity.goal`; add the three types)
- Modify: `crates/sapphire-tally-core/src/store.rs` (`ActivityFront`, `read_activity`, `activity_text`, test literals at ~201/~234)
- Modify: `crates/sapphire-tally-core/src/tally.rs` (`create_activity`, `update_activity`, `Activity` literals in tests)
- Modify: `crates/sapphire-tally-core/src/lib.rs` (re-exports)
- Modify: `README.md:104`, `README.ja.md:101` (the front matter list)

**Interfaces:**
- Produces:
  - `pub enum Bound { #[default] Above, Below }` — `Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize`, `#[serde(rename_all = "lowercase")]`, `pub fn name(self) -> &'static str` (`"above"` = a floor, more is better / `"below"` = a ceiling, less is better)
  - `pub struct Target { pub times: u32, pub bound: Bound }` — `Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize` with `#[serde(rename_all = "lowercase")]`; `FromStr`, `TryFrom<String>` / `Into<String>` (like `Color`) so a `String` field round-trips
  - `Target::status(&self, count: u64) -> Status` — `Above`: `under` while `count < times`, else `met`; `Below`: `exceeded` while `count > times`, else `met`
  - `pub enum Status { Met, Under, Exceeded }` — lowercase serde
  - `Activity.target: Option<Target>`, `ActivityUpdate.target: Option<Option<Target>>` — both where `goal` was, both `skip_serializing_if = "Option::is_none"`
  - `Tally::create_activity(&self, title: &str, unit: Option<Unit>, target: Option<Target>, color: Option<Color>, description: Option<String>)` (same arity; `goal` → `target`)

- [ ] **Step 1: Write failing tests**

In `model.rs` tests (replace `invalid_goal_is_rejected_by_parse_goal`):

```rust
#[test]
fn a_target_is_text_and_judges_a_count() {
    let below: Target = " 3-BELOW ".parse().unwrap();
    assert_eq!(below, Target { times: 3, bound: Bound::Below });
    assert_eq!(below.to_string(), "3-below");
    assert_eq!(below.status(4), Status::Exceeded); // a ceiling passed
    assert_eq!(below.status(3), Status::Met);
    let above: Target = "1-above".parse().unwrap();
    assert_eq!(above.status(0), Status::Under); // a floor not reached
    assert_eq!(above.status(1), Status::Met);
}

#[test]
fn a_bad_target_names_the_value_and_shows_the_shape() {
    for bad in ["less", "1-inside", "often", "-below"] {
        let e = bad.parse::<Target>().unwrap_err();
        assert!(e.contains(bad) && e.contains("N-bound"), "{e}");
    }
}
```

In `tally.rs` tests:

```rust
#[test]
fn a_target_writes_reads_back_and_survives_an_update() {
    let (dir, t) = tally();
    let a = t
        .create_activity(
            "タピオカミルクティー",
            None,
            Some(Target { times: 3, bound: Bound::Below }),
            None,
            None,
        )
        .unwrap();
    let text = std::fs::read_to_string(store::activity_path(dir.path(), a.id)).unwrap();
    assert!(text.contains("bound: below"), "{text}");
    assert_eq!(t.get_activity(a.id).unwrap().target, Some(a.target.unwrap()));
    let b = t
        .update_activity(a.id, ActivityUpdate { title: Some("ハーモニカ".into()), ..Default::default() })
        .unwrap();
    assert_eq!(b.target, a.target, "an update leaving target out keeps it");
    let c = t
        .update_activity(a.id, ActivityUpdate { target: Some(None), ..Default::default() })
        .unwrap();
    assert_eq!(c.target, None);
}

#[test]
fn an_activity_without_a_target_writes_no_target_key() {
    let (dir, t) = tally();
    let a = t.create_activity("筋トレ", None, None, None, None).unwrap();
    let text = std::fs::read_to_string(store::activity_path(dir.path(), a.id)).unwrap();
    assert!(!text.contains("target"), "{text}");
    assert_eq!(t.get_activity(a.id).unwrap().target, None);
}
```

- [ ] **Step 2: Run to see them fail**

Run: `cargo test -p sapphire-tally-core`
Expected: compile errors (`Target` / `Bound` / `Status` not found).

- [ ] **Step 3: Implement**

In `model.rs`, where `Goal` was:

```rust
/// Which way an activity's count should move: `times` is a floor when
/// [`Bound::Above`] (a habit to build) or a ceiling when [`Bound::Below`] (one to
/// cut down), counted in the activity's [`Unit`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Bound {
    /// More is better: `times` is a floor.
    #[default]
    Above,
    /// Less is better: `times` is a ceiling.
    Below,
}

/// How many times an activity is meant to happen in its [`Unit`], and which
/// way. `None` on an activity means only counted (数えるだけ).
///
/// Written `{times}-{bound}`, as in `3-below`: at most 3 a period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct Target {
    /// The count the `bound` is written against.
    pub times: u32,
    pub bound: Bound,
}

/// How a period's count stands against its activity's [`Target`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// The floor was reached, or the ceiling not passed.
    Met,
    /// A floor (`above`) not reached yet.
    Under,
    /// A ceiling (`below`) passed.
    Exceeded,
}

impl Target {
    /// How `count`, counted in the activity's unit, reads against this target.
    pub fn status(self, count: u64) -> Status {
        let times = u64::from(self.times);
        match self.bound {
            Bound::Above if count < times => Status::Under,
            Bound::Below if count > times => Status::Exceeded,
            _ => Status::Met,
        }
    }
}
```

`Bound::name` and the `FromStr` / `Display` / `TryFrom<String>` / `Into<String>` impls mirror `Swatch::name` and `Color`'s exactly. The parse splits on the last `-`:

```rust
let (times, bound) = lower.rsplit_once('-').ok_or_else(bad)?;
```

with the spec's error text (names the got value, shows `N-bound` as in `1-above`, and what each bound means). `Activity.goal` → `Activity.target`, `ActivityUpdate.goal` → `ActivityUpdate.target`. In `store.rs`: `ActivityFront.goal` → `target`, and the same at `read_activity` / `activity_text` / test literals. In `tally.rs`: `update_activity` gains `if let Some(target) = update.target { activity.target = target; }`. In `lib.rs`: `pub use model::{Activity, ActivityUpdate, Bound, Color, Goal→…, HeatmapBucket, Stamp, StampUpdate, Status, Swatch, Target, Unit};`. In the two READMEs, the front-matter line gains the optional `target` (`N-bound`) and `unit` is described as the period the count is measured in and drawn in.

- [ ] **Step 4: Run tests**

Run: `cargo test -p sapphire-tally-core && cargo clippy -p sapphire-tally-core --all-targets -- -D warnings`
Expected: PASS, no warnings. (Other crates will not compile until Task 2 — their `goal` fields are renamed there.)

- [ ] **Step 5: Commit**

```bash
git add crates/sapphire-tally-core README.md README.ja.md
git commit -m "feat(core)!: an activity's target: N times a unit, above or below (#56)"
```

---

### Task 2: Target over MCP — params, log entries, and what a count means

**Files:**
- Modify: `crates/sapphire-tally-mcp/src/methods.rs` (params, `activity_add`, `activity_update`, `meanings` / `meaning`, `StampEntry`, `BucketEntry`, `APP_API`, `parse_goal` → `parse_target`, tests)
- Modify: `crates/sapphire-tally-mcp/src/tools.rs` (tool descriptions, server instructions)
- Modify: `crates/sapphire-tally-server/tests/mcp_tools.rs:101` (`goal: None`)
- Modify: `desktop/src/form.rs` (`goal: Some(goal_name(...))` → `target: None` for now, so the workspace compiles; Task 3 fills it)

**Interfaces:**
- Consumes: `Bound`, `Target`, `Status`, `Activity.target`, `ActivityUpdate.target`, `Tally::create_activity(title, unit, target, color, description)` from Task 1.
- Produces:
  - `ActivityAddParams.target: Option<String>` / `ActivityUpdateParams.target: Option<String>`, both `#[serde(default)]`; add: `/// The activity's target, as \`N-bound\`: \`3-below\` is a habit cut to at most 3 its unit. Omitted means only counted.`; update adds `An empty string removes it.`
  - `pub struct StampEntry { #[serde(flatten)] pub stamp: Stamp, pub activity_title: String, #[serde(default, skip_serializing_if = "Option::is_none")] pub target: Option<Target> }`
  - `pub struct BucketEntry { #[serde(flatten)] pub bucket: HeatmapBucket, pub activity_title: String, #[serde(default, skip_serializing_if = "Option::is_none")] pub target: Option<Target>, #[serde(default, skip_serializing_if = "Option::is_none")] pub status: Option<Status> }`
  - `fn parse_target(s: &str) -> anyhow::Result<Target>` (like `parse_color`)
  - `pub const APP_API: u32 = 3;`

- [ ] **Step 1: Write failing tests** in `methods.rs` tests (rename `goal` → `target` at every existing param literal and at the 1cd8f9e test's `text.replace`):

```rust
#[test]
fn a_heatmap_bucket_carries_a_target_and_how_its_count_reads_against_it() {
    let (_dir, m) = methods();
    let tea = add(&m, "タピオカミルクティー", Some("3-below"));
    let gym = add(&m, "筋トレ", Some("2-above"));
    let only = add(&m, "ヨーヨー", None);
    for _ in 0..4 { stamp(&m, &tea); }
    stamp(&m, &gym);
    stamp(&m, &only);
    let heat: Vec<serde_json::Value> =
        serde_json::from_str(&m.heatmap(all()).unwrap()).unwrap();
    let of = |id: &str| heat.iter().find(|e| e["activity"] == id).unwrap().clone();
    assert_eq!(of(&tea)["activity_title"], "タピオカミルクティー");
    assert_eq!(of(&tea)["target"]["bound"], "below");
    assert_eq!(of(&tea)["status"], "exceeded"); // 4 of a ceiling of 3
    assert_eq!(of(&gym)["target"]["times"], 2);
    assert_eq!(of(&gym)["status"], "under"); // 1 of a floor of 2
    assert!(of(&only).get("target").is_none(), "{:?}", of(&only));
    assert!(of(&only).get("status").is_none(), "{:?}", of(&only));
}

#[test]
fn a_stamp_carries_its_activitys_target_and_no_status() {
    // one stamp is one event, not a count against anything
    let (_dir, m) = methods();
    let tea = add(&m, "タピオカミルクティー", Some("3-below"));
    stamp(&m, &tea);
    let stamps: Vec<serde_json::Value> =
        serde_json::from_str(&m.stamp_list(all()).unwrap()).unwrap();
    assert_eq!(stamps[0]["target"]["times"], 3);
    assert!(stamps[0].get("status").is_none(), "{stamps:?}");
}
```

Plus: `add` gains `target: target.map(Into::into)`; the unreadable-activity test breaks the file with `text.replace("bound: below", "bound: less")` and asserts the entry carries no `target`; a new `target: Some("less".into())` add is a tool error naming `less`; `decorated_stamp_list_still_reads_as_stamps` still compiles (the desktop reads `Vec<Stamp>`).

- [ ] **Step 2: Run to see them fail**

Run: `cargo test -p sapphire-tally-mcp`
Expected: compile errors on `target` fields / missing fields.

- [ ] **Step 3: Implement**
  - `parse_goal` → `fn parse_target(s: &str) -> anyhow::Result<Target> { s.parse::<Target>().map_err(anyhow::Error::msg) }`.
  - `activity_add`: `let target = p.target.as_deref().filter(|t| !t.trim().is_empty()).map(parse_target).transpose()?;` (as `color` already is there) and pass it to `create_activity`.
  - `activity_update`: `target: p.target.as_deref().filter(|t| !t.trim().is_empty()).map(parse_target).transpose()?,` — an empty string is no update at all; the existing field stays.
  - `meanings()` → `HashMap<GrainId, (String, Option<Target>)>` from `a.target`; `meaning()`'s fallback is `(activity.to_string(), None)`. `stamp_list` / `heatmap` pass what they get to `target`, and `heatmap` additionally `status: target.map(|t| t.status(bucket.count))`.
  - `APP_API: 3`, and its doc comment is unchanged otherwise.
  - `tools.rs`: replace every `goal` mention (activity_add/get/update/list, stamp_list, heatmap, the server instructions) with the target wording from the spec: `unit` is the period the count is measured in and drawn in; a target is `N-bound` — `above` a habit to build, `below` one to cut down; an activity without one is only counted; a `heatmap` entry's `status` is `met`, `under` (a floor not reached) or `exceeded` (a ceiling passed).

- [ ] **Step 4: Run tests**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS, no warnings (the server e2e test and the desktop compile again).

- [ ] **Step 5: Commit**

```bash
git add crates desktop/src/form.rs
git commit -m "feat(mcp)!: log entries carry the activity's target, and its count's status (#56)"
```

---

### Task 3: Target in the desktop activity form

**Files:**
- Modify: `desktop/src/form.rs` (`GOALS` / `goal_name` → `BOUNDS` / `bound_name`; `ActivityForm.goal: Goal` → `times: String` + `bound: Bound`; `edit`, `to_add_params`, `to_update_params`, the goal tests)
- Modify: `desktop/src/tally.rs:814-828` (the Goal row → a Target row) and its `Activity` literal (~1225)

**Interfaces:**
- Consumes: `Bound`, `Target` (Task 1), `ActivityAddParams.target` / `ActivityUpdateParams.target` (Task 2).
- Produces: `pub const BOUNDS: [Bound; 2] = [Bound::Above, Bound::Below];`, `pub fn bound_name(bound: Bound) -> &'static str`, `ActivityForm { times: String, bound: Bound }` (text in, parsed on read, like `StampForm`'s `when`).

- [ ] **Step 1: Write failing tests** in `form.rs` tests (replace `activity_form_sends_its_goal`):

```rust
#[test]
fn the_activity_form_reads_its_target_back() {
    let mut form = ActivityForm {
        title: "タピオカ".into(),
        times: " 3 ".into(),
        bound: Bound::Below,
        ..Default::default()
    };
    assert_eq!(form.to_add_params().unwrap().target.as_deref(), Some("3-below"));
    form.bound = Bound::Above;
    assert_eq!(
        form.to_update_params(id("012atvw")).unwrap().target.as_deref(),
        Some("3-above")
    );
    form.times.clear();
    assert_eq!(
        form.to_update_params(id("012atvw")).unwrap().target.as_deref(),
        Some(""),
        "an emptied target removes it"
    );
    form.times = "often".into();
    assert!(form.to_update_params(id("012atvw")).unwrap_err().contains("often"));
}
```

- [ ] **Step 2: Run to see it fail** — `cargo test -p sapphire-tally-desktop` (field names).

- [ ] **Step 3: Implement** — `ActivityForm` keeps `#[derive(Clone, Debug, Default, PartialEq)]` (`Bound` carries `Default`).

```rust
fn target(&self) -> Result<Option<Target>, String> {
    let times = self.times.trim();
    if times.is_empty() {
        return Ok(None);
    }
    let times = times
        .parse::<u32>()
        .map_err(|_| format!("the times must be a whole number, got `{times}`"))?;
    Ok(Some(Target { times, self.bound }))
}
```

`edit` reads `target: Option<Target>` back into `times` (`t.times.to_string()`, empty when `None`) and `bound` (`t.bound`, `Above` when `None`); `to_add_params`: `target: self.target()?.map(|t| t.to_string())`; `to_update_params`: `target: Some(self.target()?.map(|t| t.to_string()).unwrap_or_default())` — exactly the line `color` has, so an emptied target removes it. `tally.rs`: the "Goal" row keeps its pattern — label "Target", a singleline `TextEdit` on `form.times` plus two `selectable_value`s for `BOUNDS` while editing, `form.target()?.ok()` read-only display otherwise.

- [ ] **Step 4: Run tests** — `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`. Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add desktop/src
git commit -m "feat(desktop)!: choose an activity's target in its form (#56)"
```
