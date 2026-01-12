# azuki.nvim

> **WIP**: このプラグインは開発中です

Neovim 向け日本語入力プラグイン。OS IME に依存せず、Vim 操作を邪魔しない日本語入力を実現します。

## 特徴

- ライブ変換（入力中のリアルタイム変換）
- Vim 操作との自然な共存（`<Esc>` で即座にノーマルモードへ）
- SKK 辞書対応
- **Zenzai ニューラル変換**（GPT-2 ベースの高精度変換、オプション）
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

### lazy.nvim（Zenzai 有効）

Zenzai ニューラル変換を有効にする場合（推奨）：

```lua
{
  "urugus/azuki.nvim",
  build = "./scripts/setup.sh",
  config = function()
    require("azuki").setup({
      zenzai = { enabled = true }
    })
  end,
}
```

セットアップスクリプトは以下を自動で行います：
- Rust サーバーのビルド（Zenzai 機能付き）
- Zenzai モデル（約 70MB）のダウンロード

### lazy.nvim（辞書のみ）

Zenzai を使わず、SKK 辞書のみで使用する場合：

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
  run = "./scripts/setup.sh",
  config = function()
    require("azuki").setup({
      zenzai = { enabled = true }
    })
  end,
}
```

### 手動インストールの場合

プラグインマネージャーを使わない場合は、手動でセットアップしてください。

```bash
cd ~/.local/share/nvim/lazy/azuki.nvim
./scripts/setup.sh
```

または、Zenzai なしでビルドする場合：

```bash
cd ~/.local/share/nvim/lazy/azuki.nvim/server
cargo build --release
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

  -- Zenzai ニューラル変換設定
  zenzai = {
    enabled = false,                     -- ニューラル変換を有効化
    model_path = nil,                    -- モデルパス（nil で自動検出）
    inference_limit = 10,                -- 推論回数上限
    contextual = false,                  -- 文脈を考慮した変換（未実装）
  },

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

### Zenzai について

Zenzai は GPT-2 ベースのニューラルかな漢字変換エンジンです。SKK 辞書だけでは変換できない語句も、文脈を考慮して適切に変換できます。

- モデルは [Hugging Face](https://huggingface.co/Miwa-Keita/zenz-v3.1-small-gguf) から自動ダウンロードされます
- 初回の変換時にモデルがロードされるため、少し時間がかかります
- モデルサイズ: 約 70MB
- 推奨メモリ: 150MB 以上
