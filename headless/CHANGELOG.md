# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/sapphire-turtle/sapphire-tally/compare/v0.2.1...v0.3.0) - 2026-11-06

### Other

- *(deps)* [**breaking**] sapphire-framework 0.28, corundum-framework 0.11

### Changed

- **Breaking:** sapphire-framework 0.28 and corundum-framework 0.11. `update` installs only a release file signed with sapphire-turtle's key: it fetches the file's `<file>.sig` from the same release and refuses a file without one, or whose signature does not match (sapphire-turtle/sapphire-framework#402).

## [0.2.1](https://github.com/sapphire-turtle/sapphire-tally/compare/v0.2.0...v0.2.1) - 2026-11-04

### Fixed

- On Windows, `service install` no longer fails with "access is denied": the service is now a logon task for the installing user, which needs no administrator (sapphire-framework 0.27.1, corundum 0.9.1, sapphire-turtle/corundum-framework#105) ([#40](https://github.com/sapphire-turtle/sapphire-tally/pull/40))

## [0.2.0](https://github.com/sapphire-turtle/sapphire-tally/releases/tag/v0.2.0) - 2026-11-03

### Added

- **Breaking:** one desktop binary that runs the CLI too; headless, cli and server crates

### Changed

- **Breaking:** sapphire-framework 0.24. HTTP clients (`/mcp`) authenticate with API keys kept in the workspace (`<root>/.sapphire-tally/api_keys/`), managed with `sapphire-tally api-key list|add|retire|restore|rotate` (`external-device` is a hidden alias for this release). Every key has a profile: the workgroup's existing external-device tokens keep working but become **read-only**, so they may call only the reading tools (`activity_list`, `stamp_list`, `heatmap`, …); an agent that stamps needs a new key, `sapphire-tally api-key add <name> --profile read-write`. Update `sapphire-bridge` together with the tally.
- **Breaking:** the `workgroup`, `device` and `embedding` subcommands are gone: use `sapphire-bridge workgroup|device|embedding`.
- **Breaking:** sapphire-framework 0.26: `workspace init` and `select` without a directory use the app's default workspace, `<data dir>/workspaces/default`, not the current directory; pass `.` for the old behaviour.

### Fixed

- name each edition's releases, so update installs its own file

## [0.1.0](https://github.com/sapphire-turtle/sapphire-tally/releases/tag/v0.1.0) - 2026-10-25

### Added

- *(cli)* [**breaking**] print the help on a bare invocation instead of starting the server
- *(desktop)* one heatmap per activity, and delete from the windows
- *(desktop)* heatmap viewer with stamp and activity forms
- delete activities and stamps
- store records as Markdown with YAML front matter, and add get/update
- run on sapphire-framework's app server, with workspace sync and shared MCP tools
- workspace scaffold with core models and TOML store

### Other

- release with release-plz
- Merge pull request #19 from sapphire-turtle/docs/mcp-any-client
- update to sapphire-framework 0.22
- note that any MCP client works, not only sapphire-agent
- *(deps)* sapphire-framework 0.19.0; reconnect to the bridge on its own
- *(license)* license files per component, none at the root
- unify the tagline — built on sapphire-framework, human-agent collaboration
- add CONTRIBUTING (language + repo-layout conventions) and README.ja
