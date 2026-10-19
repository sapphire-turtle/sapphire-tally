# sapphire-tally

> Language: **English** | [日本語](README.ja.md)

A stamp tracker with GitHub-style heatmaps built on [sapphire-framework](https://github.com/fluo10/sapphire-framework) — file-based, local-first, made for human-agent collaboration. Stamps are an activity + timestamp + optional comment, each stored as one Markdown file with YAML front matter.

> **Works with any MCP client.** AI integration is a standard MCP server, so sapphire-tally is not tied to [sapphire-agent](https://github.com/sapphire-turtle/sapphire-agent) despite the shared name — any AI harness that can act as an MCP client (Claude Code, Claude Desktop, …) can use it.

**Under development.** One binary, `sapphire-tally`, runs the app server and carries the framework's verbs (`workspace`, `workgroup`, `mcp`, …).

## Usage

```sh
sapphire-tally serve                         # run the server (a bare `sapphire-tally` prints the help)
sapphire-tally workspace init ~/tally --sync # create a workspace, serve it, and sync it
sapphire-tally workspace select ~/tally      # switch the server to an existing workspace
sapphire-tally workspace sync enable         # turn sync on or off later (also: disable, status)
sapphire-tally mcp                           # MCP over stdio, for an agent to launch
```

Sync goes through the host's `sapphire-bridge`; without one running, the server serves its workspace unsynced. To serve MCP over HTTP at `http://127.0.0.1:3174/mcp`, set `[mcp] http = true` in the app's `server.toml`.

## Desktop app

`sapphire-tally-desktop` lists every activity with a year of its stamps as a GitHub-style heatmap — darker days have more stamps. Click a day to list its stamps, and a stamp to see, edit or delete it; **Stamp now**, **Stamp…**, **New activity** and **Details** add, change and delete records (deleting an activity deletes its stamps, after a confirmation). The Sync tab picks the server's workspace and manages the workgroup. Like the CLI, it talks to the running `sapphire-tally` server and never starts one.

```sh
cargo run -p sapphire-tally-desktop
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
| `sapphire-tally` (`cli/`) | MIT OR Apache-2.0 |
| `sapphire-tally-core` | MIT OR Apache-2.0 |
| `sapphire-tally-mcp` | MIT OR Apache-2.0 |

The desktop and mobile apps, when they land, are GPL-3.0-or-later with an app store exception, like every sapphire application's. See the `LICENSE-MIT` / `LICENSE-APACHE` files in each component's directory for the full license text.
