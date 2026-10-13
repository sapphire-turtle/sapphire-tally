# sapphire-tally（日本語）

> 言語: [English](README.md) | **日本語**

sapphire-framework上に構築された、GitHub草風ヒートマップ付きスタンプトラッカー。ファイルベース・ローカルファースト・人間とAIエージェントの協働のために作られました。スタンプは活動＋日時＋任意のコメントからなり、スタンプごとにYAMLフロントマター付きのMarkdownファイル1つとして保存されます。

**開発中です。** 単一のバイナリ `sapphire-tally` がアプリサーバーを動かし、frameworkの動詞（`workspace`・`workgroup`・`mcp` など）も備えます。

## 使い方

```sh
sapphire-tally                               # サーバーを起動（`sapphire-tally serve` と同じ）
sapphire-tally workspace init ~/tally --sync # ワークスペースを作成し、提供・同期を開始
sapphire-tally workspace select ~/tally      # 既存のワークスペースに切り替え
sapphire-tally workspace sync enable         # 後から同期を切り替え（disable / status も）
sapphire-tally mcp                           # stdio経由のMCP（エージェントから起動）
```

同期はホストの `sapphire-bridge` を経由します。bridgeが動いていなければ、同期なしでワークスペースを提供します。HTTP経由のMCP（`http://127.0.0.1:3174/mcp`）は、アプリの `server.toml` で `[mcp] http = true` を設定すると有効になります。

## デスクトップアプリ

`sapphire-tally-desktop` は、すべての種目を、それぞれの1年分のスタンプをGitHubの草のようなヒートマップ付きで一覧表示します（スタンプが多い日ほど濃い色）。日をクリックするとその日のスタンプ一覧、スタンプをクリックすると詳細表示・編集・削除ができます。**Stamp now**・**Stamp…**・**New activity**・**Details** で記録の追加・変更・削除ができます（種目を削除すると、確認のうえでそのスタンプもすべて削除されます）。Syncタブではサーバーのワークスペース選択とワークグループの管理ができます。CLIと同じく、動作中の `sapphire-tally` サーバーと通信し、自分でサーバーを起動することはありません。

```sh
cargo run -p sapphire-tally-desktop
```

## データ形式

- `<workspace>/activities/<grain-id>.md` — 種目。フロントマターに`title`（必須）と`unit`（任意: day/week/month）、本文に説明
- `<workspace>/stamps/<grain-id>.md` — スタンプ。フロントマターに`activity`と`timestamp`、本文にコメント
- ファイル名はgrain-id（7文字BASE32）。ファイル内にもidフィールドを持つが、ファイル名が正

```markdown
---
id: 012atvx
activity: 012atvw
timestamp: "2026-09-05T09:12:00+09:00"
---

横浜で買った
```

## MCPツール

`activity_add` / `activity_get` / `activity_update` / `activity_delete` / `activity_list` / `stamp_add` / `stamp_get` / `stamp_update` / `stamp_delete` / `stamp_list` / `heatmap` と、frameworkの読み取り専用ツール（`status`・`workspace_show`・`workspace_search`・`workspace_read_file`・`workspace_list_dir`・`workgroup_show`・`device_list`）
