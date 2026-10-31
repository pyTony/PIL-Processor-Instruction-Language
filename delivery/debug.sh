#!/bin/sh
set -eu
cd -- "$(dirname -- "$0")"
exec ./pil --repl --load programs/selftest.bin
