#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
profile=debug
case "${1:-}" in
  '') cargo build --locked ;;
  --release) profile=release; cargo build --locked --release ;;
  *) echo 'usage: sh tools/build.sh [--release]' >&2; exit 2 ;;
esac
build_root=${CARGO_TARGET_DIR:-target}
mkdir -p "$build_root/programs"
"$build_root/$profile/pil" --write-selftest "$build_root/programs/selftest.bin"
"$build_root/$profile/pil" --write-hello "$build_root/programs/hello.bin"
"$build_root/$profile/pil" --write-factorials "$build_root/programs/factorials.bin"
"$build_root/$profile/pil" --write-loop "$build_root/programs/loop.bin"
echo "Built $build_root/$profile/pil and $build_root/programs/selftest.bin"
