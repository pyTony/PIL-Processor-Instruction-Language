#!/bin/sh
# Inspectable native PIL proof for Termux/Android ARM64.
# This compiles fixed x86 test images into host Rust, NOT arbitrary DOS applications.
# Source emission, native compilation, and execution are deliberately separate.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
out=target/native-android-proof
source_file="$out/generated.rs"
binary="$out/generated"
trace="$out/generated.trace"
case "${1:-}" in
  --emit)
    if [ ! -x target/debug/pil ]; then
      echo 'Missing target/debug/pil: first run sh tools/build_termux.sh --compact' >&2
      exit 1
    fi
    mkdir -p "$out"
    target/debug/pil --emit-suite-rust 16 2 > "$source_file"
    echo "Generated Rust source: $source_file"
    echo "Inspect it before compiling: sed -n '1,100p' $source_file"
    ;;
  --build)
    if [ ! -s "$source_file" ]; then
      echo "Missing source: first run sh tools/native_android_proof.sh --emit" >&2
      exit 1
    fi
    # One rustc process, no DWARF, no LTO, and a low-optimization profile.
    # This is a conservative experiment, not a guarantee against memory exhaustion.
    rustc --edition=2021 -C opt-level=0 -C debuginfo=0 \
      -C codegen-units=4 -C panic=abort "$source_file" -o "$binary"
    echo "Native host executable: $binary"
    ;;
  --run)
    if [ ! -x "$binary" ]; then
      echo "Missing executable: first run sh tools/native_android_proof.sh --build" >&2
      exit 1
    fi
    "$binary" "$trace" verify
    echo "Generated trace: $trace"
    ;;
  *)
    echo 'usage: sh tools/native_android_proof.sh --emit|--build|--run' >&2
    exit 2
    ;;
esac
