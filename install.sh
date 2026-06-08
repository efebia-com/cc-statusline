#!/usr/bin/env sh
# Install cc-statusline (statusline + ctx-left + bridge-ls) and wire it into Claude Code.
#
# From-source: requires Rust (https://rustup.rs) and a C compiler for the bundled
# SQLite (Linux: gcc/clang; macOS: Xcode Command Line Tools). Run from a clone:
#
#     sh install.sh
#
# Honors $CLAUDE_CONFIG_DIR and $CARGO_HOME if set.
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
CLAUDE_DIR="${CLAUDE_CONFIG_DIR:-$HOME/.claude}"
CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"

if ! command -v cargo >/dev/null 2>&1; then
    echo "error: 'cargo' not found — install Rust from https://rustup.rs" >&2
    exit 1
fi
if ! command -v python3 >/dev/null 2>&1; then
    echo "error: 'python3' not found — needed only to merge settings.json" >&2
    exit 1
fi

echo "==> Building + installing binaries (statusline, ctx-left, bridge-ls) into $CARGO_BIN"
cargo install --path "$SCRIPT_DIR" --force

echo "==> Installing the ctx-left skill into $CLAUDE_DIR/skills/ctx-left"
mkdir -p "$CLAUDE_DIR/skills/ctx-left"
cp "$SCRIPT_DIR/skill/ctx-left/SKILL.md" "$CLAUDE_DIR/skills/ctx-left/SKILL.md"

echo "==> Pointing statusLine.command at $CARGO_BIN/statusline (merging settings.json)"
python3 - "$CLAUDE_DIR/settings.json" "$CARGO_BIN/statusline" <<'PY'
import json, os, sys
settings, command = sys.argv[1], sys.argv[2]
data = {}
if os.path.exists(settings):
    with open(settings, encoding="utf-8") as f:
        data = json.load(f)          # preserve every existing key
data.setdefault("statusLine", {})
data["statusLine"]["type"] = "command"
data["statusLine"]["command"] = command
os.makedirs(os.path.dirname(settings), exist_ok=True)
with open(settings, "w", encoding="utf-8") as f:
    json.dump(data, f, indent=2)
    f.write("\n")
PY

echo "OK. Restart Claude Code to load the statusline.   Try it:  ctx-left --all   ·   bridge-ls"
