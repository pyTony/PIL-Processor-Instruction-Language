#!/bin/sh
set -eu
cd -- "$(dirname -- "$0")"
exec ./pil-proof --repl --load programs/selftest.bin
