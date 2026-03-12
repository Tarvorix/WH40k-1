#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WEB_DIR="$(dirname "$SCRIPT_DIR")"
ENGINE_DIR="$(dirname "$WEB_DIR")/engine"

echo "Building WASM module..."
cd "$ENGINE_DIR"
wasm-pack build crates/wasm_api --target web --out-dir "$WEB_DIR/wasm-pkg"
echo "WASM build complete -> $WEB_DIR/wasm-pkg/"
