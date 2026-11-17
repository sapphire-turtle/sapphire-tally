# Activity goal: increase / decrease / track

Issue: #53. Short design (a small change).

## Why

Stamps for a habit the user wants to cut down (bubble tea) are read by an AI
agent as "times I resisted". The MCP log tools (`stamp_list`, `heatmap`)
return only the activity's id, so the agent reads counts without knowing the
activity's title or which way the user wants the count to move, and nothing
says a stamp means the thing *happened*.

## Goals

- Each activity says whether more is better (`increase`), less is better
  (`decrease`), or it is only counted (`track`).
- Every MCP log result carries that, next to the activity's title, so an agent
  cannot read a count without its meaning.
- The tool and server descriptions say a stamp records that the activity
  happened.

Non-goals: streaks, targets or scores derived from the goal; colours (#54).

## Approaches

- **Per-entry fields on log results (picked).** Each stamp / bucket in
  `stamp_list` and `heatmap` gets `activity_title` and `goal`. The meaning
  travels with every count, even when an agent only calls one tool.
- Rely on `activity_list`: no output change, but the agent has to join by id
  itself, which is what fails today.
- Wrap results as `{ activities: [...], stamps: [...] }`: less repetition, but
  still a join, and a breaking shape change for every consumer.

## Design

- `sapphire-tally-core`: `enum Goal { Increase, Decrease, Track }`
  (lowercase serde, `Track` default, `Goal::parse_goal` like `Unit`).
  `Activity.goal: Option<Goal>` in front matter, skipped when `None`; `None`
  reads as `track`, so existing files are unchanged. `ActivityUpdate.goal`.
- `sapphire-tally-mcp`:
  - `activity_add` / `activity_update` take an optional `goal` string.
  - `stamp_list` / `heatmap` answer entries with the stamp's / bucket's fields
    flattened plus `activity_title` and `goal` (resolved, always present).
    The desktop deserializes `stamp_list` into `Vec<Stamp>`; serde ignores the
    extra fields, so it keeps working.
  - Tool descriptions and server instructions: "A stamp records that the
    activity happened once. `goal` says which way the user wants the count to
    move: `decrease` means fewer is better."
  - `APP_API` 1 → 2 (shipped in the same release as #54).
- Desktop: a goal combo box in the activity form, beside `unit`.

## Left to the builder

Names of the output structs, where the title lookup lives (core vs. mcp), and
test layout.
