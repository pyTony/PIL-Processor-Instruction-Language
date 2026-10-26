#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
sh tools/build.sh --release >&2
build_root=${CARGO_TARGET_DIR:-target}
mode=--mandelbrot
if [ "${1:-}" = "--compare" ]; then mode=--compare-mandelbrot; shift; fi
exec "$build_root/release/pil-proof" "$mode" "$@"
