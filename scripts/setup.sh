#!/bin/bash
# azuki.nvim setup script
# This script builds the Rust server with Zenzai support and downloads the neural model

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "[azuki] Starting setup..."

# 1. Build Rust server with zenzai feature
echo "[azuki] Building server with Zenzai support..."
cd "$PROJECT_ROOT/server"

if command -v cargo &> /dev/null; then
    cargo build --release --features zenzai
    echo "[azuki] Server built successfully"
else
    echo "[azuki] Warning: Rust toolchain not found, skipping server build"
    echo "[azuki] Please install Rust from https://rustup.rs/ and run this script again"
fi

# 2. Create model directory
MODEL_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/azuki/models"
mkdir -p "$MODEL_DIR"

# 3. Download Zenzai model if not present
MODEL_PATH="$MODEL_DIR/zenz-v3.1-small.gguf"
MODEL_URL="https://huggingface.co/Miwa-Keita/zenz-v3.1-small-gguf/resolve/main/ggml-model-Q5_K_M.gguf"

if [ ! -f "$MODEL_PATH" ]; then
    echo "[azuki] Downloading Zenzai model (~70MB)..."
    if command -v curl &> /dev/null; then
        curl -L --progress-bar -o "$MODEL_PATH" "$MODEL_URL"
        echo "[azuki] Model downloaded successfully"
    elif command -v wget &> /dev/null; then
        wget --show-progress -O "$MODEL_PATH" "$MODEL_URL"
        echo "[azuki] Model downloaded successfully"
    else
        echo "[azuki] Warning: Neither curl nor wget found"
        echo "[azuki] Please download the model manually from:"
        echo "        $MODEL_URL"
        echo "        and place it at: $MODEL_PATH"
    fi
else
    echo "[azuki] Model already exists at $MODEL_PATH"
fi

# 4. Create dictionary directory
DICT_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/azuki/dict"
mkdir -p "$DICT_DIR"

echo "[azuki] Setup complete!"
echo ""
echo "To use Zenzai neural conversion, add this to your setup:"
echo ""
echo '  require("azuki").setup({'
echo '    zenzai = { enabled = true }'
echo '  })'
