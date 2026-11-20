# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/sapphire-turtle/sapphire-tally/compare/sapphire-tally-core-v0.1.3...sapphire-tally-core-v0.2.0) - 2026-11-20

### Added

- *(core)* [**breaking**] an activity's target: N times a unit, above or below ([#56](https://github.com/sapphire-turtle/sapphire-tally/pull/56))
- *(core)* an activity's colour, a swatch or oklch(C H) ([#54](https://github.com/sapphire-turtle/sapphire-tally/pull/54))
- *(core)* an activity's goal: increase, decrease or track ([#53](https://github.com/sapphire-turtle/sapphire-tally/pull/53))

### Other

- hold the time tests in any host time zone
- [**breaking**] times are jiff's, not chrono's (RUSTSEC-2026-0543)

### Breaking

- A stamp's time is `jiff::Timestamp`, not chrono's `DateTime<FixedOffset>`: chrono is unmaintained (RUSTSEC-2026-0543). `Stamp`, `StampUpdate` and `Tally::add_stamp` change type; `time` has the helpers (`now`, `local`, `local_date`, `parse`, `format`).

### Changed

- Stamps are written as before, in this host's offset (`2026-09-05T09:12:34+09:00`), and releases on chrono still read them. A time without an offset is read in this host's time zone, where it made the stamp unreadable before.
- Stamps count toward the day they fall on in this host's time zone (`list_stamps`, `heatmap`), not the day in the offset they were recorded in.
- sapphire-framework 0.29, corundum-framework 0.13.

## [0.1.3](https://github.com/sapphire-turtle/sapphire-tally/compare/sapphire-tally-core-v0.1.2...sapphire-tally-core-v0.1.3) - 2026-11-06

### Changed

- sapphire-framework 0.28 (sapphire-turtle/sapphire-framework#402).

## [0.1.2](https://github.com/sapphire-turtle/sapphire-tally/compare/sapphire-tally-core-v0.1.1...sapphire-tally-core-v0.1.2) - 2026-11-04

### Other

- update Cargo.toml dependencies

## [0.1.1](https://github.com/sapphire-turtle/sapphire-tally/compare/sapphire-tally-core-v0.1.0...sapphire-tally-core-v0.1.1) - 2026-11-03

### Other

- update Cargo.toml dependencies

## [0.1.0](https://github.com/sapphire-turtle/sapphire-tally/releases/tag/sapphire-tally-core-v0.1.0) - 2026-10-25

### Added

- delete activities and stamps
- store records as Markdown with YAML front matter, and add get/update
- run on sapphire-framework's app server, with workspace sync and shared MCP tools
- Tally facade with open/create/list/add and heatmap aggregation
- workspace scaffold with core models and TOML store

### Other

- release with release-plz
- *(license)* license files per component, none at the root
