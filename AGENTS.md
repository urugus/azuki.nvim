# Repository Guidelines

## Project Structure & Module Organization

現状、このリポジトリは設計資料中心です（`DESIGN.md`, `AGENTS.md`）。実装が追加されたら、基本的に以下の構成を前提に整理します。

- `lua/azuki/`: Neovim プラグイン本体（例: `init.lua`, `config.lua`, `romaji.lua`, `server.lua`, `ui.lua`）
- `server/`: 変換サーバ（Rust）。`Cargo.toml` と `src/` を配置
- `README.md`: インストール手順・使い方
- `DESIGN.md`: プロトコル（length-prefixed JSON）とライブ変換方針の仕様

## Build, Test, and Development Commands

現状はビルド/テスト用のスクリプトが未整備です。追加された場合は、README に記載し、可能なら `make` 経由で統一します。例:

- `cd server && cargo build --release`: Rust サーバをビルド
- `cd server && cargo test`: サーバ側のテスト（導入時）
- `cd server && cargo fmt`: Rust のフォーマット（導入時）
- `cd server && cargo clippy -- -D warnings`: Rust の静的解析（導入時）
- `nvim --clean +"set rtp+=." +"lua require('azuki').setup{}"`: ローカルで読み込み確認（実装後）

## Coding Style & Naming Conventions

- インデント: Lua は 2 スペース、Rust は標準のフォーマットに従う（導入された formatter/linter が最優先）
- 命名: Lua モジュールは `lua/azuki/<name>.lua`（小文字 + `_` を基本）。公開 API は `require('azuki')` から辿れる形にする
- 設計変更: 実装は `DESIGN.md` の意図（プロセス分離・stdio 通信）から逸脱する場合、先に設計更新を含める
- 互換性: プロトコルや既定パス（例: `stdpath('data') .. '/azuki/bin/azuki-server'`）を変える場合は、移行方針も併記する

## Testing Guidelines

現状はテスト基盤がありません。導入する場合の目安:

- Lua: 文字種変換（ローマ字→ひらがな）や状態遷移をユニットテスト化
- Rust: length-prefixed JSON のフレーミング、`seq` の取り扱い、辞書/学習データロードをテスト化
- 命名例: `*_spec.lua` / Rust の標準に合わせる

## Commit & Pull Request Guidelines

このリポジトリは現時点で Git 履歴がないため、慣習は未確定です。提案:

- コミット: Conventional Commits（例: `feat: add server stdio protocol` / `fix: handle cancel key`）
- PR: 目的・設計への影響・動作確認手順（コマンドや手順）を必須にする。UI 変更がある場合はスクリーンショット/短い動画を添付

## Security & Configuration Tips

- 外部プロセス（`azuki-server`）起動経路・実行ファイルパスは明示し、ユーザー入力をそのままシェル解釈しない
- stdio プロトコルはサイズ上限を設け（例: 数MBまで）、壊れたメッセージで固まらないようにタイムアウト/再起動方針を用意する
- 個人辞書・学習データ・モデル等はリポジトリにコミットせず、`.gitignore` 対象にする
