# sapphire-tally（日本語）

> 言語: [English](README.md) | **日本語**

sapphire-framework上に構築された、GitHub草風ヒートマップ付きスタンプトラッカー。ファイルベース・ローカルファースト・人間とAIエージェントの協働のために作られました。スタンプは活動＋日時＋任意のコメントからなり、スタンプごとにYAMLフロントマター付きのMarkdownファイル1つとして保存されます。

> **任意のMCPクライアントから使えます。** AIとの連携は汎用的なMCPサーバーとして実装しているため、名前が同じ[sapphire-agent](https://github.com/sapphire-turtle/sapphire-agent)専用ではありません。MCPクライアント機能を持つAIハーネス（Claude Code、Claude Desktopなど）であれば何でも連携できます。

**開発中です。** 1つのコマンド `sapphire-tally` がアプリサーバーを動かし、frameworkの動詞（`workspace`・`workgroup`・`mcp` など）も備えます。

## インストール

2つの版の**どちらか1つ**を入れてください。どちらも PATH に同じコマンド `sapphire-tally` を置きます。
デスクトップ版のバイナリはヘッドレス版のコマンド（`serve`・`mcp`・`workspace` など）もすべて実行できるので、ヘッドレス版の上位互換です。

| 版 | 用途 | 入れ方 |
|---|---|---|
| デスクトップ版（`sapphire-tally-desktop`） | GUI を使うふだんの利用 | Windows・macOS・Linux のインストーラー（準備中） |
| ヘッドレス版（`sapphire-tally-headless`） | GUI のないホストやサーバー | インストールスクリプト、cargo-binstall、cargo |

両方を入れると `sapphire-tally` が2つになります。PATH 上のものが実行したものと違うときは、
`sapphire-tally status` が警告します。

### ヘッドレス版：インストールスクリプト

```sh
# Linux, macOS
curl -fsSL https://raw.githubusercontent.com/sapphire-turtle/sapphire-framework/main/install/install.sh | sh -s -- tally
```

```powershell
# Windows
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/sapphire-turtle/sapphire-framework/main/install/install.ps1))) tally
```

`sapphire-tally` と、同期に使う `sapphire-bridge` を `~/.local/bin` にインストールします。オプションは
[インストールスクリプトの README](https://github.com/sapphire-turtle/sapphire-framework/tree/main/install/README.ja.md) を参照してください。

### ヘッドレス版：cargo-binstall か cargo

```sh
cargo binstall sapphire-tally-headless
# またはビルドする
cargo install sapphire-tally-headless
```

どちらも `sapphire-tally` バイナリを入れます（0.1 は `sapphire-tally` という crate 名で公開していました）。

## 使い方

```sh
sapphire-tally serve                         # サーバーを起動（引数なしの `sapphire-tally` はヘルプを表示）
sapphire-tally workspace init ~/tally --sync # ワークスペースを作成し、提供・同期を開始
sapphire-tally workspace select ~/tally      # 既存のワークスペースに切り替え
sapphire-tally workspace sync enable         # 後から同期を切り替え（disable / status も）
sapphire-tally mcp                           # stdio経由のMCP（エージェントから起動）
```

同期はホストの `sapphire-bridge` を経由します。bridgeが動いていなければ、同期なしでワークスペースを提供します。HTTP経由のMCP（`http://127.0.0.1:3174/mcp`）は、アプリの `config.toml` で `[http.mcp] enabled = true` を設定すると（`sapphire-tally config set http.mcp.enabled=true`）有効になります。

## デスクトップアプリ

`sapphire-tally-desktop` は、すべての種目を、それぞれの1年分のスタンプをGitHubの草のようなヒートマップ付きで一覧表示します（スタンプが多い日ほど濃い色）。日をクリックするとその日のスタンプ一覧、スタンプをクリックすると詳細表示・編集・削除ができます。**Stamp now**・**Stamp…**・**New activity**・**Details** で記録の追加・変更・削除ができます（種目を削除すると、確認のうえでそのスタンプもすべて削除されます）。Syncタブではサーバーのワークスペース選択とワークグループの管理ができます。CLIと同じく、ウィンドウは動作中の `sapphire-tally` サーバーと通信し、自分でサーバーを起動することはありません。

このバイナリはCLIも兼ねます。引数なしならウィンドウを開き、サブコマンド（`serve`・`mcp`・`status`・`workspace` など）を付ければ `sapphire-tally` と同じ処理をします。インストーラー（準備中）はこれを `sapphire-tally` として PATH に置くので、ヘッドレス版と並べずに、その代わりに入れてください。それまでは [Releases](https://github.com/sapphire-turtle/sapphire-tally/releases) のバイナリ（`sapphire-tally-desktop-<os>-<arch>`）をそのまま実行できます。

```sh
cargo run -p sapphire-tally-desktop             # ウィンドウ
cargo run -p sapphire-tally-desktop -- status   # コマンド（`sapphire-tally status` と同じ）
```

## 構成

```
sapphire-tally/
├── headless/                # sapphire-tally-headless：GUIなしの sapphire-tally バイナリ
├── desktop/                 # デスクトップGUI（egui）。同じコマンドも実行できる
└── crates/
    ├── sapphire-tally-cli/    # 両方の版が使うコマンドライン
    ├── sapphire-tally-server/ # アプリサーバー（`sapphire-tally serve`）
    ├── sapphire-tally-core/   # データモデルとMarkdownストア
    └── sapphire-tally-mcp/    # tallyのサーバーメソッドと、それを呼ぶMCPツール
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

## ライセンス

| コンポーネント | ライセンス |
|-----------|---------|
| `sapphire-tally-cli` | MIT OR Apache-2.0 |
| `sapphire-tally-core` | MIT OR Apache-2.0 |
| `sapphire-tally-headless` | MIT OR Apache-2.0 |
| `sapphire-tally-mcp` | MIT OR Apache-2.0 |
| `sapphire-tally-server` | MIT OR Apache-2.0 |
| `sapphire-tally-desktop` | GPL-3.0-or-later、[App Storeマーケットプレイス例外](desktop/LICENSE.App-Store-Exception)付き |

デスクトップ版のバイナリはコマンドラインとサーバーの MIT / Apache-2.0 のコードを含むため、バイナリ全体は GPL-3.0-or-later（同じ例外付き）で配布されます。

モバイル版も、ほかの sapphire アプリと同じく GPL-3.0-or-later（App Storeマーケットプレイス例外付き）になります。各コンポーネントのディレクトリにある`LICENSE`（または`LICENSE-MIT` / `LICENSE-APACHE`）ファイルに完全なライセンス文書があります。
