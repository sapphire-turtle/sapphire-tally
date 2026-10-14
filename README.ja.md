# sapphire-tally（日本語）

> 言語: [English](README.md) | **日本語**

sapphire-framework上に構築された、GitHub草風ヒートマップ付きスタンプトラッカー。ファイルベース・ローカルファースト・人間とAIエージェントの協働のために作られました。スタンプは活動＋日時＋任意の一行コメントからなり、スタンプごとにプレーンなTOMLファイル1つとして保存されます。

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

## データ形式

- `<workspace>/activities/<grain-id>.toml` — 種目（title必須、unit任意: day/week/month）
- `<workspace>/stamps/<grain-id>.toml` — スタンプ（activity/timestamp必須、comment任意）
- ファイル名はgrain-id（7文字BASE32）。ファイル内にもidフィールドを持つが、ファイル名が正

## MCPツール

`activity_add` / `activity_list` / `stamp_add` / `stamp_list` / `heatmap` と、frameworkの読み取り専用ツール（`status`・`workspace_show`・`workspace_search`・`workspace_read_file`・`workspace_list_dir`・`workgroup_show`・`device_list`）
