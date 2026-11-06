# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/sapphire-turtle/sapphire-tally/compare/v0.2.1...v0.3.0) - 2026-11-06

### Added

- *(desktop)* the macOS icon on the brand's tile

### Fixed

- *(desktop)* the Windows executable carries the app icon, in the brand colour

### Other

- *(deps)* [**breaking**] sapphire-framework 0.28, corundum-framework 0.11
- Merge origin/main into fix/windows-exe-icon

### Added

- On macOS, the app bundle put in `~/Applications` when Sapphire Tally is added to the app menu has the mark on the brand's tile, as the package's icns, rather than the bare mark on the system's plate (corundum-framework 0.11).

### Changed

- **Breaking:** sapphire-framework 0.28 and corundum-framework 0.11. `update` installs only a release file signed with sapphire-turtle's key: it fetches the file's `<file>.sig` from the same release and refuses a file without one, or whose signature does not match (sapphire-turtle/sapphire-framework#402).

## [0.2.1](https://github.com/sapphire-turtle/sapphire-tally/compare/v0.2.0...v0.2.1) - 2026-11-04

### Fixed

- On Windows, keeping the app server running (the first start's offer, or `service install`) no longer fails with "access is denied": the service is now a logon task for the installing user, which needs no administrator (sapphire-framework 0.27, corundum 0.9, sapphire-turtle/corundum-framework#105).
- On Windows, applying an update from the app's window installs it and starts the app again; before, the app only closed and was not updated (sapphire-framework 0.27.1, corundum 0.9.1, sapphire-turtle/corundum-framework#109). An app at v0.2.0 or earlier still hits this when it applies this update: install v0.2.1 by hand once, from its `_x64-setup.exe` ([#40](https://github.com/sapphire-turtle/sapphire-tally/pull/40))

## [0.2.0](https://github.com/sapphire-turtle/sapphire-tally/compare/v0.1.0...v0.2.0) - 2026-11-03

### Added

- **Breaking:** Sapphire Tally is distributed as an installer per OS, made with cargo-packager ([#39](https://github.com/sapphire-turtle/sapphire-tally/pull/39)): `sapphire-tally-desktop_<version>_x64-setup.exe`, `_aarch64.dmg` and `_x86_64.AppImage`. The installed app updates itself from the latest release, checking each update's signature first.
- **Breaking:** one desktop binary that runs the CLI too; headless, cli and server crates
- list the picked activity's or day's stamps in the side pane
- the app icon

### Changed

- **Breaking:** sapphire-framework 0.24. HTTP clients (`/mcp`) authenticate with API keys kept in the workspace (`<root>/.sapphire-tally/api_keys/`), managed with `sapphire-tally api-key list|add|retire|restore|rotate` (`external-device` is a hidden alias for this release). Every key has a profile: the workgroup's existing external-device tokens keep working but become **read-only**, so they may call only the reading tools (`activity_list`, `stamp_list`, `heatmap`, …); an agent that stamps needs a new key, `sapphire-tally api-key add <name> --profile read-write`. Update `sapphire-bridge` together with the tally.
- **Breaking:** the `workgroup`, `device` and `embedding` subcommands are gone: use `sapphire-bridge workgroup|device|embedding`.
- Settings link to the Sapphire Bridge window for the workgroup, devices and embedding (sapphire-framework 0.24), and gain an API keys page.
- **Breaking:** sapphire-framework 0.26: `workspace init` and `select` without a directory use the app's default workspace, `<data dir>/workspaces/default`, not the current directory; pass `.` for the old behaviour.

### Fixed

- name each edition's releases, so update installs its own file

## [0.1.0](https://github.com/sapphire-turtle/sapphire-tally/releases/tag/desktop-v0.1.0) - 2026-10-25

### Added

- *(desktop)* lay the window out with the framework's Shell
- *(desktop)* one heatmap per activity, and delete from the windows
- *(desktop)* heatmap viewer with stamp and activity forms

### Other

- release with release-plz
- update to sapphire-framework 0.22
- *(desktop)* sapphire-framework 0.21; start through gui::app::run
