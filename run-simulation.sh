#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$SCRIPT_DIR/tessera-grid-rs"

if [[ ! -f "$APP_DIR/Cargo.toml" ]]; then
  echo "Error: Could not find Cargo.toml in $APP_DIR" >&2
  exit 1
fi

cd "$APP_DIR"
# Launch the GUI, or pass CLI flags through for headless runs, e.g.:
#   ./run-simulation.sh --steps 100 --png out.png
cargo run --release -- "$@"
