# Activity Goal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Each activity carries `goal: increase | decrease | track`, and every MCP log result (`stamp_list`, `heatmap`) carries the activity's title and goal so an agent reads counts with their meaning.

**Architecture:** `Goal` lives in `sapphire-tally-core` next to `Unit` and is stored in the activity's front matter. `sapphire-tally-mcp` takes it on add/update and decorates log entries with `activity_title` + resolved `goal` via flattened wrapper structs. The desktop form gets a combo box.

**Tech Stack:** Rust 2024, serde (YAML front matter, JSON), rmcp, egui (desktop).

**Spec:** `docs/superpowers/specs/2026-11-16-activity-goal-design.md` (issue #53)

## Global Constraints

- Values are exactly `increase`, `decrease`, `track` (lowercase in files, JSON and tool args); parsing ignores case and surrounding whitespace, like `Unit::parse_unit`.
- An activity without `goal` reads as `track`; front matter omits `goal` when it is `None`, so existing files are unchanged byte for byte.
- Log entries always include `goal` (resolved, never null) and `activity_title`.
- `APP_API` goes from `1` to `2`.
- Description copy (tool + server instructions) must contain: "A stamp records that the activity happened once." and "`decrease` means fewer is better."
- Run `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` before each commit.

## Review Focus

1. A bad goal string (`"less"`) → tool error naming the value and listing `increase/decrease/track`. (Task 1 + Task 2 tests)
2. `activity_update` without `goal` keeps the existing goal; with `goal` changes it. (Task 1 test)
3. An activity file written before this change (no `goal` key) → `stamp_list` / `heatmap` say `"goal": "track"`. (Task 2 test)
4. `heatmap` / `stamp_list` with no `activity` filter across two activities → each entry carries its own activity's title and goal. (Task 2 test)
5. The desktop still deserializes the decorated `stamp_list` answer into `Vec<Stamp>`. (Task 2 test)

---

### Task 1: `Goal` in core

**Files:**
- Modify: `crates/sapphire-tally-core/src/model.rs` (add `Goal`, `Activity.goal`, `ActivityUpdate.goal`)
- Modify: `crates/sapphire-tally-core/src/store.rs` (`ActivityFront.goal`, `read_activity`, `activity_text`, test literals at ~201/~234)
- Modify: `crates/sapphire-tally-core/src/tally.rs` (`create_activity`, `update_activity`, existing test call sites)
- Modify: `crates/sapphire-tally-core/src/lib.rs` (re-export `Goal`)

**Interfaces:**
- Produces:
  - `pub enum Goal { Increase, Decrease, #[default] Track }` — `Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize`, `#[serde(rename_all = "lowercase")]`
  - `Goal::parse_goal(s: &str) -> Result<Goal, String>`; error text: ``goal must be one of increase/decrease/track, got `{other}` ``
  - `Activity.goal: Option<Goal>` (`#[serde(default, skip_serializing_if = "Option::is_none")]`), placed after `unit`
  - `ActivityUpdate.goal: Option<Goal>`
  - `Tally::create_activity(&self, title: &str, unit: Option<Unit>, goal: Option<Goal>, description: Option<String>) -> Result<Activity>`

- [ ] **Step 1: Write failing tests**

In `model.rs` tests:
```rust
#[test]
fn invalid_goal_is_rejected_by_parse_goal() {
    let e = Goal::parse_goal("less").unwrap_err();
    assert!(e.contains("less") && e.contains("increase/decrease/track"), "{e}");
    assert_eq!(Goal::parse_goal(" Decrease ").unwrap(), Goal::Decrease);
}
```
In `tally.rs` tests:
```rust
#[test]
fn goal_roundtrips_and_update_keeps_it_when_left_out() {
    let (_d, t) = tally();
    let a = t.create_activity("タピオカ", None, Some(Goal::Decrease), None).unwrap();
    assert_eq!(t.get_activity(a.id).unwrap().goal, Some(Goal::Decrease));
    let a2 = t.update_activity(a.id, ActivityUpdate { title: Some("タピオカミルクティー".into()), ..Default::default() }).unwrap();
    assert_eq!(a2.goal, Some(Goal::Decrease));
    let a3 = t.update_activity(a.id, ActivityUpdate { goal: Some(Goal::Track), ..Default::default() }).unwrap();
    assert_eq!(t.get_activity(a3.id).unwrap().goal, Some(Goal::Track));
}

#[test]
fn an_activity_without_goal_writes_no_goal_key() {
    let (d, t) = tally();
    let a = t.create_activity("筋トレ", None, None, None).unwrap();
    let text = std::fs::read_to_string(store::activity_path(d.path(), a.id)).unwrap();
    assert!(!text.contains("goal"), "{text}");
    assert_eq!(t.get_activity(a.id).unwrap().goal, None);
}
```

- [ ] **Step 2: Run to see them fail**

Run: `cargo test -p sapphire-tally-core`
Expected: compile errors (`Goal` not found, `create_activity` takes 3 args).

- [ ] **Step 3: Implement** the Produces items. Mirror `unit` exactly in `ActivityFront`, `read_activity`, `update_activity`. Add `None` for `goal` at every existing `create_activity(...)` call and `Activity { .. }` literal in core tests.

- [ ] **Step 4: Run tests**

Run: `cargo test -p sapphire-tally-core`
Expected: PASS. (Other crates won't compile yet; that's Task 2.)

- [ ] **Step 5: Commit**

```bash
git add crates/sapphire-tally-core
git commit -m "feat(core): an activity's goal: increase, decrease or track (#53)"
```

---

### Task 2: Goal over MCP, and log entries that carry it

**Files:**
- Modify: `crates/sapphire-tally-mcp/src/methods.rs` (params, `activity_add`, `activity_update`, `stamp_list`, `heatmap`, `APP_API`, `parse_goal` helper, tests)
- Modify: `crates/sapphire-tally-mcp/src/tools.rs` (descriptions, server instructions)
- Modify: `crates/sapphire-tally-server/tests/mcp_tools.rs:98` (`goal: None`)
- Modify: `desktop/src/form.rs` `to_add_params` / `to_update_params` (`goal: None` for now, so the workspace compiles; Task 3 fills it)

**Interfaces:**
- Consumes: `Goal`, `Goal::parse_goal`, `Activity.goal`, `ActivityUpdate.goal`, `Tally::create_activity(title, unit, goal, description)` from Task 1.
- Produces:
  - `ActivityAddParams.goal: Option<String>` and `ActivityUpdateParams.goal: Option<String>`, both `#[serde(default)]`, doc comment: `/// increase (more is better), decrease (fewer is better) or track (only counted). track when omitted.`
  - `pub struct StampEntry { #[serde(flatten)] pub stamp: Stamp, pub activity_title: String, pub goal: Goal }` — `Debug, Serialize, Deserialize`
  - `pub struct BucketEntry { #[serde(flatten)] pub bucket: HeatmapBucket, pub activity_title: String, pub goal: Goal }` — same derives
  - `pub const APP_API: u32 = 2;`

- [ ] **Step 1: Write failing tests** in `methods.rs` tests (add `goal: None` to existing param literals):

```rust
#[test]
fn log_entries_carry_each_activitys_title_and_goal() {
    let (_dir, m) = methods();
    let add = |title: &str, goal: Option<&str>| {
        let v: serde_json::Value = serde_json::from_str(&m.activity_add(ActivityAddParams {
            title: title.into(), unit: None, goal: goal.map(Into::into), description: None,
        }).unwrap()).unwrap();
        v["id"].as_str().unwrap().to_string()
    };
    let tea = add("タピオカミルクティー", Some("decrease"));
    let gym = add("筋トレ", None);
    for a in [&tea, &gym] {
        m.stamp_add(StampAddParams { activity: a.clone(), timestamp: Some("2026-09-05T09:00:00+09:00".into()), comment: None }).unwrap();
    }
    let all = QueryParams { activity: None, from: None, to: None };
    let stamps: Vec<serde_json::Value> = serde_json::from_str(&m.stamp_list(all.clone()).unwrap()).unwrap();
    let heat: Vec<serde_json::Value> = serde_json::from_str(&m.heatmap(all).unwrap()).unwrap();
    for entries in [&stamps, &heat] {
        let of = |id: &str| entries.iter().find(|e| e["activity"] == id).unwrap().clone();
        assert_eq!(of(&tea)["activity_title"], "タピオカミルクティー");
        assert_eq!(of(&tea)["goal"], "decrease");
        assert_eq!(of(&gym)["activity_title"], "筋トレ");
        assert_eq!(of(&gym)["goal"], "track");
    }
}

#[test]
fn decorated_stamp_list_still_reads_as_stamps() {
    // the desktop deserializes stamp_list into Vec<Stamp>
    let (_dir, m) = methods();
    /* add one activity + one stamp as above */
    let s: Vec<Stamp> = serde_json::from_str(&m.stamp_list(QueryParams { activity: None, from: None, to: None }).unwrap()).unwrap();
    assert_eq!(s.len(), 1);
}

#[test]
fn invalid_goal_returns_tool_error() {
    let (_dir, m) = methods();
    let e = m.activity_add(ActivityAddParams { title: "x".into(), unit: None, goal: Some("less".into()), description: None }).unwrap_err();
    assert!(e.contains("less"), "{e}");
}
```
(`QueryParams` needs `Clone`; add the derive.)

- [ ] **Step 2: Run to see them fail**

Run: `cargo test -p sapphire-tally-mcp`
Expected: compile errors on `goal` fields / missing `StampEntry`.

- [ ] **Step 3: Implement**
  - `activity_add` / `activity_update` parse `goal` with a `parse_goal(Option<&str>) -> anyhow::Result<Option<Goal>>` helper shaped like `parse_unit`.
  - `stamp_list` / `heatmap`: load `list_activities()` once into a `HashMap<GrainId, &Activity>`, map each stamp / bucket to `StampEntry` / `BucketEntry` with `goal: activity.goal.unwrap_or_default()`. An entry whose activity is missing from the map is skipped (core already skips orphans; this keeps it total).
  - `APP_API = 2`, update its neighbouring comment if it mentions a version.
  - `tools.rs`: `activity_add` / `activity_update` descriptions mention `goal` (increase/decrease/track, default track). `stamp_list`, `heatmap` descriptions and the server instructions add: "A stamp records that the activity happened once. Each entry carries the activity's title and goal: `increase` means more is better, `decrease` means fewer is better, `track` is only counted."
  - Add `goal: None` in `crates/sapphire-tally-server/tests/mcp_tools.rs` and `desktop/src/form.rs`.

- [ ] **Step 4: Run tests**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS, no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates desktop/src/form.rs
git commit -m "feat(mcp)!: log entries carry the activity's title and goal (#53)"
```

---

### Task 3: Goal in the desktop activity form

**Files:**
- Modify: `desktop/src/form.rs` (`GOALS`, `goal_name`, `ActivityForm.goal`, `edit`, `to_add_params`, `to_update_params`, tests)
- Modify: `desktop/src/tally.rs:794-810` (a "Goal" row after "Unit", same pattern; `Activity` literal at ~1198 gets `goal: None`)

**Interfaces:**
- Consumes: `Goal` (Task 1), `ActivityAddParams.goal` / `ActivityUpdateParams.goal` (Task 2).
- Produces: `pub const GOALS: [Goal; 3] = [Goal::Track, Goal::Increase, Goal::Decrease];`, `pub fn goal_name(goal: Goal) -> &'static str` returning `"track"` / `"increase"` / `"decrease"`, `ActivityForm.goal: Goal`.

- [ ] **Step 1: Write failing test** in `form.rs` tests:
```rust
#[test]
fn activity_form_sends_its_goal() {
    let mut form = ActivityForm { title: "タピオカ".into(), ..Default::default() };
    assert_eq!(form.to_add_params().unwrap().goal.as_deref(), Some("track"));
    form.goal = Goal::Decrease;
    assert_eq!(form.to_update_params(GrainId::random()).unwrap().goal.as_deref(), Some("decrease"));
}
```

- [ ] **Step 2: Run to see it fail**

Run: `cargo test -p sapphire-tally-desktop form`
Expected: compile error (`goal` field missing on `ActivityForm`).

- [ ] **Step 3: Implement** the Produces items; `edit` uses `activity.goal.unwrap_or_default()`; both param builders send `Some(goal_name(self.goal).to_owned())`. Add the combo box row (id salt `"activity_goal"`, label `"Goal"`, read-only label when not editing), mirroring the Unit row.

- [ ] **Step 4: Run tests**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS, no warnings.

- [ ] **Step 5: Commit**

```bash
git add desktop
git commit -m "feat(desktop): choose an activity's goal in its form (#53)"
```
