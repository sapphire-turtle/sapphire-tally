# Activity target: how many times, at least or at most

Issue: #56. Small design (a change to what #53 shipped two days ago, not yet in a release).

## Why

A `goal` of `increase` / `decrease` / `track` says which way a count should move, but not how far, and not against what. Read through MCP, `タピオカミルクティー` (a habit to cut down) and `家計簿記帳` (a habit to build) look the same: a title, a direction, and a number of stamps with nothing saying what number that count should reach. So the agent still has to guess at the one thing it is being asked to nudge about: has this count passed what it should?

The period it should be counted in is the activity's own `unit`, which `goal` does not carry at all.

## Goals

- An activity may say exactly how many times, and whether that is a floor or a ceiling: `1-above` for 筋トレ (build), `3-below` for タピオカミルクティー (cut down), counted in its `unit`.
- `unit` means one thing: **the period the count is measured in** — the period stamps are counted and drawn in, and the period a target is written against.
- An activity without a target is only counted (数えるだけ), which is what every activity today is. That is the default and needs no field.
- `goal` is deleted. A target is a superset of it (the direction is the bound), except for `track`, which its absence now says. Two fields that can disagree is one field too many.
- Every MCP log entry carries what its count means: `target` where the activity has one, and `status` where there is a count to read it against.

Non-goals: streaks, scores, targets that change over time, a per-target unit other than the activity's own, extra tool arguments beyond `target`.

## Design

### sapphire-tally-core

- `enum Bound { #[default] Above, Below }` — `#[serde(rename_all = "lowercase")]`. `Above`: `times` is a floor — more is better. `Below`: `times` is a ceiling — less is better. The default exists only so `ActivityForm` can derive `Default`; a stored `Target` is always explicit.
- `struct Target { times: u32, bound: Bound }` — `Copy` + serde with `#[serde(rename_all = "lowercase")]`, so the front matter reads:

  ```yaml
  target:
    times: 3
    bound: below
  ```

  `times` is the count the `bound` is written against, counted in the activity's `unit`.
- `Target` implements `Display` + `FromStr` + `TryFrom<String>` (like `Color`, not like `Unit::parse_unit` — `Target` has two parts to it). The text form is `"{times}-{bound}"` (`3-below`); parsing ignores case and surrounding whitespace. Error text names the got value and shows the shape:

  ```text
  target must look like `N-bound`, as in `1-above`, with bound `above` (a floor, more is better) or `below` (a ceiling, less is better), got `{other}`
  ```

  `Display` writes that same text form back, so a tool arg round-trips.
- `enum Status { Met, Under, Exceeded }` — lowercase serde; how a period's count stands against its activity's target:

  | `bound` | condition | `status` |
  |---|---|---|
  | `above` | `count < times` | `under` — a floor not yet reached |
  | `above` | `count >= times` | `met` |
  | `below` | `count > times` | `exceeded` — a ceiling passed |
  | `below` | `count <= times` | `met` |

- `impl Target { pub fn status(&self, count: u64) -> Status }` — the table above. It lives on `Target`, not on the count: what a count was compared against is what makes it `met` rather than `under`.
- `Activity.target: Option<Target>` replaces `Activity.goal`; `ActivityUpdate.target: Option<Option<Target>>` replaces `ActivityUpdate.goal` (an empty tool arg removes it, exactly as `color` does today), both placed where `goal` was and both `skip_serializing_if = "Option::is_none"`, so a counted-only activity's file has no `target` line and is unchanged byte for byte.
- `Unit`'s doc comment gains that it is the period a target is counted in. `Unit` itself does not change.
- `lib.rs` re-exports `Bound`, `Status`, `Target`.

### sapphire-tally-mcp

- `ActivityAddParams.target` / `ActivityUpdateParams.target`: `Option<String>`, the text form, `""` removing on update as `color` does. `methods::parse_goal` is deleted; `parse_target` takes its place (same shape, `Option<&str> -> anyhow::Result<Option<Goal→Target>>`).
- `activity_get` / `activity_list` keep returning the activity as stored — the `target` there is a mapping and already self-explanatory. No new entry type for them.
- Log entries keep their flattened shape (the #53 one) and gain what the count means:
  - `StampEntry` gains `target: Option<Target>`, skipped when `None`. One stamp is one event, not a count, so it carries no `status`.
  - `BucketEntry` gains `target: Option<Target>` and `status: Option<Status>` — the latter from `target.status(bucket.count)`, derived from the bucket's own `count`. Both skipped when `None`.
  - `meanings()` / `meaning()` (the 1cd8f9e fix) carry `(String, Option<Target>)`; an unreadable activity's stamps still count, under its id as title and with no `target` (what cannot be read cannot be judged).
- Tool descriptions, server instructions, and the params' `target` doc comments say: a stamp records that the activity happened once; a `target` is counted in the activity's `unit`, `above` a habit to build, `below` one to cut down; an activity without one is only counted; `status` says how a period's count stands against its target.
- `APP_API` 2 → 3 (the meaning of `goal` → `target` is incompatible; an old client is refused at connect, not mid-call).

### desktop

- `form::GOALS` / `goal_name` deleted; `form::BOUNDS: [Bound; 2]` and `bound_name` take their place. `ActivityForm` keeps its text-in, parse-on-read shape (like `StampForm`'s `when`): `times: String` (empty is only counted) + `bound: Bound`, and `target()` builds the `Option<Target>`. The window's Goal row becomes a Target row: a count text field plus an above/below toggle.

### docs

- `README.md` / `README.ja.md`: the front matter gains the optional `target`; `unit` is described as the period the count is measured in and drawn in.

## Migration

No `goal:` line exists in any file on disk yet (all four activities predate #53), and serde_yaml ignores unknown keys on read anyway, so a leftover one would be ignored. Nothing is renamed; no `unit` is re-read.
