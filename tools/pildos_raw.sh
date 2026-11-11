#!/bin/sh
# Put an interactive POSIX terminal into character mode for the Windows or
# Unix compiled PILdos shell. Always restore the terminal on normal exit.
set -u
engine=${PILDOS_ENGINE:-programs/compiled/pildos.exe}
saved=
if [ -t 0 ]; then
    saved=$(stty -g) || exit 1
    stty -icanon -echo min 1 time 0 || exit 1
    restore_terminal() { stty "$saved"; }
    trap restore_terminal 0
    trap 'exit 130' INT
    trap 'exit 143' TERM
    trap 'exit 129' HUP
fi
if [ -t 1 ]; then
    "$engine" --screen "$@"
else
    "$engine" "$@"
fi
status=$?
exit "$status"
