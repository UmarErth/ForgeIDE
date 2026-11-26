#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
npm ci
cargo install tauri-cli --version '^2' --locked
python3 scripts/create-platform-icons.py
cargo tauri build --bundles app,dmg
