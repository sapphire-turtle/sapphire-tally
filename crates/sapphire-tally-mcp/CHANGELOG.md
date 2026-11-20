# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/sapphire-turtle/sapphire-tally/compare/sapphire-tally-mcp-v0.2.2...sapphire-tally-mcp-v0.3.0) - 2026-11-20

### Added

- *(mcp)* [**breaking**] log entries carry the activity's target, and its count's status ([#56](https://github.com/sapphire-turtle/sapphire-tally/pull/56))
- *(mcp)* set and clear an activity's colour ([#54](https://github.com/sapphire-turtle/sapphire-tally/pull/54))
- *(mcp)* [**breaking**] log entries carry the activity's title and goal ([#53](https://github.com/sapphire-turtle/sapphire-tally/pull/53))

### Fixed

- *(mcp)* clarify target update and preserve absent form target ([#56](https://github.com/sapphire-turtle/sapphire-tally/pull/56))
- an even heatmap ramp, a colour that survives saving, and an empty colour on add ([#54](https://github.com/sapphire-turtle/sapphire-tally/pull/54))
- *(mcp)* stamps of an unreadable activity still count in the logs ([#53](https://github.com/sapphire-turtle/sapphire-tally/pull/53))

### Other

- hold the time tests in any host time zone
- [**breaking**] times are jiff's, not chrono's (RUSTSEC-2026-0543)

### Changed

- A stamp's `timestamp` may leave out the offset: it is then a time in the server's time zone.
- sapphire-framework 0.29, corundum-framework 0.13.

## [0.2.2](https://github.com/sapphire-turtle/sapphire-tally/compare/sapphire-tally-mcp-v0.2.1...sapphire-tally-mcp-v0.2.2) - 2026-11-06

### Changed

- sapphire-framework 0.28 (sapphire-turtle/sapphire-framework#402).

## [0.2.1](https://github.com/sapphire-turtle/sapphire-tally/compare/sapphire-tally-mcp-v0.2.0...sapphire-tally-mcp-v0.2.1) - 2026-11-04

### Other

- update Cargo.toml dependencies

## [0.2.0](https://github.com/sapphire-turtle/sapphire-tally/compare/sapphire-tally-mcp-v0.1.0...sapphire-tally-mcp-v0.2.0) - 2026-11-03

### Other

- *(deps)* [**breaking**] sapphire-framework 0.26

## [0.1.0](https://github.com/sapphire-turtle/sapphire-tally/releases/tag/sapphire-tally-mcp-v0.1.0) - 2026-10-25

### Added

- delete activities and stamps
- store records as Markdown with YAML front matter, and add get/update
- run on sapphire-framework's app server, with workspace sync and shared MCP tools

### Other

- *(license)* license files per component, none at the root
