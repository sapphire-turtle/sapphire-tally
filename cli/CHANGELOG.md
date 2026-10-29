# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/sapphire-turtle/sapphire-tally/compare/v0.1.0...v0.1.1) - 2026-10-29

### Other

- call the shared workflows from sapphire-turtle/.github@v1

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
