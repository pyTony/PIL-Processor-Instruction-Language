#!/bin/sh
# Resolve only the script location; input paths stay relative to the caller.
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec python3 "$script_dir/tools/build_dine.py" "$@"
