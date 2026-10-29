# sapphire-tally

> Language: **English** | [日本語](README.ja.md)

A stamp tracker with GitHub-style heatmaps built on [sapphire-framework](https://github.com/fluo10/sapphire-framework) — file-based, local-first, made for human-agent collaboration. Stamps are an activity + timestamp + optional comment, each stored as one Markdown file with YAML front matter.

> **Works with any MCP client.** AI integration is a standard MCP server, so sapphire-tally is not tied to [sapphire-agent](https://github.com/sapphire-turtle/sapphire-agent) despite the shared name — any AI harness that can act as an MCP client (Claude Code, Claude Desktop, …) can use it.

**Under development.** One command, `sapphire-tally`, runs the app server and carries the framework's verbs (`workspace`, `workgroup`, `mcp`, …).

## Installation

Install **one** of the two editions. Both put the same command, `sapphire-tally`, on
PATH: the desktop app's binary also runs every command of the headless one (`serve`,
`mcp`, `workspace`, …), so it is a superset of it.

| Edition | For | How |
|---|---|---|
| Desktop app (`sapphire-tally-desktop`) | Everyday use, with the GUI | Installer for Windows, macOS and Linux (in preparation) |
| Headless (`sapphire-tally-headless`) | Hosts and servers without a GUI | Install script, cargo-binstall or cargo |

Installing both leaves two `sapphire-tally` commands; `sapphire-tally status` warns
when the one on PATH is not the one you ran.

### Headless: install script

```sh
# Linux, macOS
curl -fsSL https://raw.githubusercontent.com/sapphire-turtle/sapphire-framework/main/install/install.sh | sh -s -- tally
```

```powershell
# Windows
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/sapphire-turtle/sapphire-framework/main/install/install.ps1))) tally
```

This installs `sapphire-tally` together with `sapphire-bridge`, which it syncs
through, into `~/.local/bin`. See the
[install script's README](https://github.com/sapphire-turtle/sapphire-framework/tree/main/install) for the options.

### Headless: cargo-binstall or cargo

```sh
cargo binstall sapphire-tally-headless
# or build it
cargo install sapphire-tally-headless
```

Either installs the `sapphire-tally` binary. (0.1 was published as the
`sapphire-tally` crate.)

## Usage

```sh
sapphire-tally serve                         # run the server (a bare `sapphire-tally` prints the help)
sapphire-tally workspace init ~/tally --sync # create a workspace, serve it, and sync it
sapphire-tally workspace select ~/tally      # switch the server to an existing workspace
sapphire-tally workspace sync enable         # turn sync on or off later (also: disable, status)
sapphire-tally mcp                           # MCP over stdio, for an agent to launch
```

Sync goes through the host's `sapphire-bridge`; without one running, the server serves its workspace unsynced. To serve MCP over HTTP at `http://127.0.0.1:3174/mcp`, set `[http.mcp] enabled = true` in the app's `config.toml` (`sapphire-tally config set http.mcp.enabled=true`).

## Desktop app

`sapphire-tally-desktop` lists every activity with a year of its stamps as a GitHub-style heatmap — darker days have more stamps. Click a day to list its stamps, and a stamp to see, edit or delete it; **Stamp now**, **Stamp…**, **New activity** and **Details** add, change and delete records (deleting an activity deletes its stamps, after a confirmation). The Sync tab picks the server's workspace and manages the workgroup. Like the CLI, the window talks to the running `sapphire-tally` server and never starts one.

Its binary is also the CLI: run bare it opens the window, and with a subcommand (`serve`, `mcp`, `status`, `workspace`, …) it does what `sapphire-tally` does. Its installer (in preparation) puts it on PATH as `sapphire-tally`, so install it instead of the headless edition, not beside it. Until then, the binaries on the [Releases](https://github.com/sapphire-turtle/sapphire-tally/releases) page (`sapphire-tally-desktop-<os>-<arch>`) can be run directly.

```sh
cargo run -p sapphire-tally-desktop             # the window
cargo run -p sapphire-tally-desktop -- status   # a command, as `sapphire-tally status`
```

## Layout

```
sapphire-tally/
├── headless/                # sapphire-tally-headless: the `sapphire-tally` binary without the GUI
├── desktop/                 # Desktop GUI (egui), which runs the same commands
└── crates/
    ├── sapphire-tally-cli/    # The command line both editions run
    ├── sapphire-tally-server/ # The app server (`sapphire-tally serve`)
    ├── sapphire-tally-core/   # Data model and Markdown store
    └── sapphire-tally-mcp/    # The tally's server methods and the MCP tools that call them
```

## Data format

- `<workspace>/activities/<grain-id>.md` — an activity: `title` (required) and `unit` (optional: day/week/month) in front matter, the description as the body
- `<workspace>/stamps/<grain-id>.md` — a stamp: `activity` and `timestamp` in front matter, the comment as the body
- Filenames are grain-ids (7-char BASE32). Each file also carries an `id` field, but the filename is authoritative.

```markdown
---
id: 012atvx
activity: 012atvw
timestamp: "2026-09-05T09:12:00+09:00"
---

Bought it in Yokohama.
```

## MCP tools

`activity_add` / `activity_get` / `activity_update` / `activity_delete` / `activity_list` / `stamp_add` / `stamp_get` / `stamp_update` / `stamp_delete` / `stamp_list` / `heatmap`, plus the framework's read-only tools (`status`, `workspace_show`, `workspace_search`, `workspace_read_file`, `workspace_list_dir`, `workgroup_show`, `device_list`)

## License

| Component | License |
|-----------|---------|
| `sapphire-tally-cli` | MIT OR Apache-2.0 |
| `sapphire-tally-core` | MIT OR Apache-2.0 |
| `sapphire-tally-headless` | MIT OR Apache-2.0 |
| `sapphire-tally-mcp` | MIT OR Apache-2.0 |
| `sapphire-tally-server` | MIT OR Apache-2.0 |
| `sapphire-tally-desktop` | GPL-3.0-or-later, with [App Store marketplace exception](desktop/LICENSE.App-Store-Exception) |

The desktop app's binary includes the MIT/Apache-2.0 code of the command line and the
server, so the binary as a whole is distributed under GPL-3.0-or-later (with the same
exception).

The mobile app, when it lands, is GPL-3.0-or-later with an app store exception too, like every sapphire application's. See the `LICENSE` (or `LICENSE-MIT` / `LICENSE-APACHE`) file in each component's directory for the full license text.
