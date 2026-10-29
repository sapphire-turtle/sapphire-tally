# sapphire-tally-headless

The headless edition of [sapphire-tally](https://github.com/sapphire-turtle/sapphire-tally) — a stamp tracker with GitHub-style heatmaps —
for hosts without a GUI. Its one binary, `sapphire-tally`, runs the app server that serves
and syncs a tally, carries the framework's verbs (`workspace`, `workgroup`, `status`, …),
and is the MCP server for AI agents (`sapphire-tally mcp`). The desktop app's binary runs
the same commands and is put on PATH under the same name.

## Installation

### Install script

```sh
# Linux, macOS
curl -fsSL https://raw.githubusercontent.com/sapphire-turtle/sapphire-framework/main/install/install.sh | sh -s -- tally
```

```powershell
# Windows
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/sapphire-turtle/sapphire-framework/main/install/install.ps1))) tally
```

This installs `sapphire-tally` together with `sapphire-bridge`, which it syncs through,
into `~/.local/bin`. See the
[install script's README](https://github.com/sapphire-turtle/sapphire-framework/tree/main/install) for the options.

### From crates.io

```bash
cargo binstall sapphire-tally-headless   # the release file
cargo install sapphire-tally-headless    # or build it
```

Either installs the `sapphire-tally` binary. The desktop app's binary runs the same
commands; install only one of the two.

## Usage

See the [repository's README](https://github.com/sapphire-turtle/sapphire-tally#usage).

## License

MIT OR Apache-2.0.
