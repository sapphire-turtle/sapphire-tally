# sapphire-tally

> Language: **English** | [日本語](README.ja.md)

A stamp tracker with GitHub-style heatmaps built on [sapphire-framework](https://github.com/fluo10/sapphire-framework) — file-based, local-first, made for human-agent collaboration. Stamps are an activity + timestamp + optional comment, each stored as one Markdown file with YAML front matter.

**Under development.** One binary, `sapphire-tally`, runs the app server and carries the framework's verbs (`workspace`, `workgroup`, `mcp`, …).

## Usage

```sh
sapphire-tally                               # run the server (same as `sapphire-tally serve`)
sapphire-tally workspace init ~/tally --sync # create a workspace, serve it, and sync it
sapphire-tally workspace select ~/tally      # switch the server to an existing workspace
sapphire-tally workspace sync enable         # turn sync on or off later (also: disable, status)
sapphire-tally mcp                           # MCP over stdio, for an agent to launch
```

Sync goes through the host's `sapphire-bridge`; without one running, the server serves its workspace unsynced. To serve MCP over HTTP at `http://127.0.0.1:3174/mcp`, set `[mcp] http = true` in the app's `server.toml`.

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
