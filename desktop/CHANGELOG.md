# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Breaking:** sapphire-framework 0.24. HTTP clients (`/mcp`) authenticate with API keys kept in the workspace (`<root>/.sapphire-tally/api_keys/`), managed with `sapphire-tally api-key list|add|retire|restore|rotate` (`external-device` is a hidden alias for this release). Every key has a profile: the workgroup's existing external-device tokens keep working but become **read-only**, so they may call only the reading tools (`activity_list`, `stamp_list`, `heatmap`, …); an agent that stamps needs a new key, `sapphire-tally api-key add <name> --profile read-write`. Update `sapphire-bridge` together with the tally.
- **Breaking:** the `workgroup`, `device` and `embedding` subcommands are gone: use `sapphire-bridge workgroup|device|embedding`.
- Settings link to the Sapphire Bridge window for the workgroup, devices and embedding (sapphire-framework 0.24), and gain an API keys page.

## [0.1.0](https://github.com/sapphire-turtle/sapphire-tally/releases/tag/desktop-v0.1.0) - 2026-10-25

### Added

- *(desktop)* lay the window out with the framework's Shell
- *(desktop)* one heatmap per activity, and delete from the windows
- *(desktop)* heatmap viewer with stamp and activity forms

### Other

- release with release-plz
- update to sapphire-framework 0.22
- *(desktop)* sapphire-framework 0.21; start through gui::app::run
