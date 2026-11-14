#!/bin/sh
# Linux and Android (Termux + Termux-x11) counterpart of PILdos.vbs:
# opens the Dine DOS terminal with FreeCOM, drive D: and AUTOEXEC.BAT.
# Extra arguments go to tools/dos_terminal.py, e.g. --ignore-unassigned-out.
cd "$(dirname "$0")" || exit 1
if [ "${1:-}" = "--rust-text" ]; then
    shift
    exec "${PILDOS_ENGINE:-./target/release/pil}" --dos-text "$@"
fi
if [ -f "${PILDOS_CONFIG:-pildos.conf}" ]; then
    exec python3 tools/dos_terminal.py "$@"
fi
drive=${PILDRIVE:-driveD}
exec python3 tools/dos_terminal.py --country 358 --mount "$drive" \
    --args '/K D:\AUTOEXEC.BAT' images/FD14LITE.img::COMMAND.COM "$@"
