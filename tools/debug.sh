#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
sh tools/build.sh >&2
build_root=${CARGO_TARGET_DIR:-target}
case "${1:-}" in
  --check) exec "$build_root/debug/pil" --selftest ;;
  '') exec "$build_root/debug/pil" --repl --load "$build_root/programs/selftest.bin" ;;
  *) exec "$build_root/debug/pil" --repl --load "$1" ;;
esac
