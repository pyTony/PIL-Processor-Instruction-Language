#!/bin/sh
# Memory-constrained native Termux build; no GUI toolkit or X11 required.
# Build only the CLI binary, not the optional Dine shared library.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
mode=debug
compact=false
common=false
for arg in "$@"; do
  case "$arg" in
    --release) mode=release ;;
    --compact) compact=true ;;
    --common) common=true ;;
    *) echo 'usage: sh tools/build_termux.sh [--release] [--compact|--common]' >&2; exit 2 ;;
  esac
done

if [ "$compact" = true ] && [ "$common" = true ]; then
  echo 'choose at most one of --compact and --common' >&2
  exit 2
fi

# Keep the regular build unchanged; Termux skips the C ABI shared library,
# compact removes generated PIL forms, and common keeps only the most used forms.
if [ "$compact" = true ]; then
  set -- --no-default-features --features compact-interpreter
elif [ "$common" = true ]; then
  set -- --no-default-features --features common-forms
else
  set -- --no-default-features
fi

# One rustc at a time; no incremental cache or large DWARF debug sections.
# These overrides are local to this command, never system-wide.
export CARGO_BUILD_JOBS=1
export CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
export CARGO_PROFILE_RELEASE_DEBUG=0
export CARGO_PROFILE_RELEASE_LTO=off
export CARGO_PROFILE_DEV_CODEGEN_UNITS=4
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=4

if [ "$mode" = release ]; then
  cargo build --locked --bin pil -j 1 --release "$@"
else
  cargo build --locked --bin pil -j 1 "$@"
fi
echo "Ready: ./target/$mode/pil"
echo "Run the built binary directly (not cargo run): ./target/$mode/pil --help"
