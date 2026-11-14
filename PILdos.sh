#!/bin/sh
# Linux and Android (Termux) counterpart of PILdos.vbs.
# Default: the Rust PILdos setup screen (pil --pildos) in this terminal; it
# edits pildos.conf (or PILDOS_CONFIG) and F10 starts the configured program.
# Extra arguments go to pil --pildos, e.g. a CONFIG path.
#   --rust-text ...  pil --dos-text ... instead
#   --python ...     the older Python/Tk terminal (tools/dos_terminal.py ...)
# PILDOS_ENGINE chooses the pil executable.
cd "$(dirname "$0")" || exit 1

engine=${PILDOS_ENGINE:-}
if [ -z "$engine" ]; then
    for candidate in ./pil ./target/release/pil ./target/debug/pil; do
        if [ -x "$candidate" ]; then engine=$candidate; break; fi
    done
fi

case "${1:-}" in
--python)
    shift
    if [ -f "${PILDOS_CONFIG:-pildos.conf}" ]; then
        exec python3 tools/dos_terminal.py "$@"
    fi
    drive=${PILDRIVE:-driveD}
    exec python3 tools/dos_terminal.py --country 358 --mount "$drive" \
        --args '/K D:\AUTOEXEC.BAT' images/FD14LITE.img::COMMAND.COM "$@"
    ;;
esac

if [ -z "$engine" ]; then
    echo "No PIL engine in $(pwd). Run: cargo build --release --locked" >&2
    exit 1
fi
if [ "${1:-}" = "--rust-text" ]; then
    shift
    exec "$engine" --dos-text "$@"
fi
exec "$engine" --pildos "$@"
