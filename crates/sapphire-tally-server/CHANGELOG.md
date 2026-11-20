# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0](https://github.com/sapphire-turtle/sapphire-tally/compare/sapphire-tally-server-v0.3.0...sapphire-tally-server-v0.4.0) - 2026-11-20

### Added

- *(mcp)* [**breaking**] log entries carry the activity's target, and its count's status ([#56](https://github.com/sapphire-turtle/sapphire-tally/pull/56))
- *(mcp)* set and clear an activity's colour ([#54](https://github.com/sapphire-turtle/sapphire-tally/pull/54))
- *(mcp)* [**breaking**] log entries carry the activity's title and goal ([#53](https://github.com/sapphire-turtle/sapphire-tally/pull/53))

### Other

- hold the time tests in any host time zone

## [0.3.0](https://github.com/sapphire-turtle/sapphire-tally/compare/sapphire-tally-server-v0.2.1...sapphire-tally-server-v0.3.0) - 2026-11-06

### Changed

- sapphire-framework 0.28 (sapphire-turtle/sapphire-framework#402).

## [0.2.1](https://github.com/sapphire-turtle/sapphire-tally/compare/sapphire-tally-server-v0.2.0...sapphire-tally-server-v0.2.1) - 2026-11-04

### Other

- update Cargo.toml dependencies

## [0.2.0](https://github.com/sapphire-turtle/sapphire-tally/releases/tag/sapphire-tally-server-v0.2.0) - 2026-11-03

### Added

- [**breaking**] one desktop binary that runs the CLI too; headless, cli and server crates

### Fixed

- name each edition's releases, so update installs its own file
