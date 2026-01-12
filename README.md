# azuki.nvim

> **WIP**: このプラグインは開発中です

Neovim 向け日本語入力プラグイン。OS IME に依存せず、Vim 操作を邪魔しない日本語入力を実現します。

## 特徴

- ライブ変換（入力中のリアルタイム変換）
- Vim 操作との自然な共存（`<Esc>` で即座にノーマルモードへ）
- SKK 辞書対応
- プロセス分離による安定性（変換サーバーがクラッシュしても Neovim は影響を受けない）

## 必要環境

- Neovim >= 0.9.0
- SKK 辞書（以下のパスを自動検索）
  - `$XDG_DATA_HOME/azuki/dict/SKK-JISYO.L`
  - `~/.local/share/azuki/dict/SKK-JISYO.L`
  - `~/.azuki/dict/SKK-JISYO.L`
  - `/usr/share/skk/SKK-JISYO.L`
  - `/usr/local/share/skk/SKK-JISYO.L`
  - または環境変数 `AZUKI_DICTIONARY` で指定
- サーバービルド時: Rust toolchain

## インストール

### lazy.nvim

```lua
{
  "urugus/azuki.nvim",
  build = "cd server && cargo build --release",
  config = function()
    require("azuki").setup()
  end,
}
```

### packer.nvim

```lua
use {
  "urugus/azuki.nvim",
  run = "cd server && cargo build --release",
  config = function()
    require("azuki").setup()
  end,
}
```

### サーバーのビルド

プラグインインストール後、サーバーをビルドします。

```bash
cd ~/.local/share/nvim/lazy/azuki.nvim/server
cargo build --release
```

ビルド済みバイナリを配置する場合:

```bash
mkdir -p ~/.local/share/nvim/azuki/bin
cp target/release/azuki-server ~/.local/share/nvim/azuki/bin/
```

## 使い方

### 基本操作

| キー | 動作 |
|------|------|
| `<C-j>` | 日本語入力モード ON/OFF |
| `<Space>` | 次の変換候補 |
| `<S-Space>` | 前の変換候補 |
| `<Enter>` | 現在の候補で確定 |
| `<C-g>` | 変換キャンセル（ひらがなに戻す） |
| `<Esc>` | 入力モード OFF + ノーマルモード |

### コマンド

| コマンド | 説明 |
|----------|------|
| `:AzukiStart` | サーバーを起動 |
| `:AzukiStop` | サーバーを停止 |
| `:AzukiStatus` | 状態を表示 |
| `:AzukiToggle` | 日本語入力モードを切替 |
| `:AzukiTest [読み]` | 変換テスト |

## 設定

```lua
require("azuki").setup({
  -- サーバーパス（nil で自動検出）
  server_path = nil,

  -- ライブ変換のデバウンス時間（ミリ秒）
  debounce_ms = 30,

  -- 日本語入力モード切替キー
  toggle_key = "<C-j>",

  -- ライブ変換の有効/無効
  live_conversion = true,

  -- ハイライトグループ
  highlight = {
    pending = "AzukiPending",           -- 未確定文字
    selected = "AzukiSelected",         -- 選択中候補
    segment = "AzukiSegment",           -- セグメント
    current_segment = "AzukiCurrentSegment", -- 現在のセグメント
  },

  -- 学習機能（未実装）
  learning = true,
  learning_file = vim.fn.stdpath("data") .. "/azuki/learning.json",
})
```
