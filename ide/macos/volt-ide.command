#!/bin/bash
# Double-click in Finder, or run from Terminal.
set -e
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

if [[ -x "$ROOT/target/release/voltc" ]]; then
  exec "$ROOT/target/release/voltc" ide "$@"
elif [[ -x "$ROOT/target/debug/voltc" ]]; then
  exec "$ROOT/target/debug/voltc" ide "$@"
else
  echo "Building voltc then starting the IDE..."
  exec cargo run -- ide "$@"
fi
