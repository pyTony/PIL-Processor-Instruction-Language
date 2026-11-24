#!/usr/bin/env bash
# Build and exercise the MCC Interim 1.0+ (Linux 1.0.4) 386 environment
# under PIL/Dine. Intended for Linux and Termux.
#
# Run from any directory:
#   bash tools/mcc/build-termux.sh [--all|--prepare|--install|--gcc|--mandel|--svgalib]
#
# --all is the default. A pre-existing hd40.img is preserved: --all skips
# the destructive installer stage and continues with GCC/Mandelbrot/svgalib.
# To reinstall, move images/mcc-1.0/hd40.img aside yourself, then use --all.

set -Eeuo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

IMAGE_DIR="images/mcc-1.0"
BASE_URL="https://raw.githubusercontent.com/youngd24/oldlinux/main"
MCC_URL="$BASE_URL/mcc-linux/MCC-1.0"
SVGA_URL="$BASE_URL/slackware/slackware-2.0.0/source/d/svgalib/svgalib111.tar.gz"

DO_PREPARE=0
DO_INSTALL=0
DO_GCC=0
DO_MANDEL=0
DO_SVGALIB=0
EXPLICIT_STAGE=0

usage() {
  cat <<'EOF'
Usage: bash tools/mcc/build-termux.sh [stage]

Stages:
  --all       Prepare, install MCC Base if the disk is new, then build/test
              gcc 2.5.8, the 80387 Mandelbrot, and svgalib 1.1 (default).
  --prepare   Build PIL and download/prepare all MCC and compiler media.
  --install   Install MCC Base onto a NEW 40 MB disk image.
  --gcc       Install gcc 2.5.8 packages onto an already-installed MCC disk.
  --mandel    Compile and time the fixed-point and 80387 Mandelbrot tests.
  --svgalib   Build svgalib 1.1 and run its VGA tests.
  -h, --help  Show this help.

The hard-disk image is persistent and is never silently replaced. To start
from scratch, move images/mcc-1.0/hd40.img out of the way before --all.
EOF
}

for arg in "$@"; do
  case "$arg" in
    --all) DO_PREPARE=1; DO_INSTALL=1; DO_GCC=1; DO_MANDEL=1; DO_SVGALIB=1; EXPLICIT_STAGE=1 ;;
    --prepare) DO_PREPARE=1; EXPLICIT_STAGE=1 ;;
    --install) DO_INSTALL=1; EXPLICIT_STAGE=1 ;;
    --gcc) DO_GCC=1; EXPLICIT_STAGE=1 ;;
    --mandel) DO_MANDEL=1; EXPLICIT_STAGE=1 ;;
    --svgalib) DO_SVGALIB=1; EXPLICIT_STAGE=1 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $arg" >&2; usage >&2; exit 2 ;;
  esac
done

if (( EXPLICIT_STAGE == 0 )); then
  DO_PREPARE=1; DO_INSTALL=1; DO_GCC=1; DO_MANDEL=1; DO_SVGALIB=1
fi

die() { echo "ERROR: $*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "Missing '$1'. On Termux install it with: pkg install $2"; }

need cargo rust
need curl curl
need python3 python
need gzip gzip
need sha256sum coreutils

echo "==> Repository: $ROOT"
echo "==> Host: $(uname -srm)"

# The same locked release build used by the Linux runs in PRs #307-#310.
echo "==> Building PIL/Dine (release, locked)"
cargo build --release --locked
PIL="$ROOT/target/release/pil"
[[ -x "$PIL" ]] || die "Expected executable not found: $PIL"

mkdir -p "$IMAGE_DIR"

download() {
  local url="$1" out="$2" expected_size="${3:-}"
  if [[ -s "$out" ]]; then
    echo "    Keeping existing $(basename "$out")"
  else
    echo "    Downloading $(basename "$out")"
    curl --fail --location --retry 3 --retry-delay 2 --output "$out.part" "$url"
    mv "$out.part" "$out"
  fi
  if [[ -n "$expected_size" ]]; then
    local actual
    actual="$(wc -c < "$out" | tr -d ' ')"
    [[ "$actual" == "$expected_size" ]] || die "$(basename "$out"): expected $expected_size bytes, got $actual"
  fi
}

prepare_media() {
  echo "==> Downloading MCC Interim 1.0+ and build sources"
  download "$MCC_URL/images/nocdboot.gz" "$IMAGE_DIR/nocdboot.gz"
  download "$MCC_URL/images/root.gz" "$IMAGE_DIR/root.gz"
  download "$MCC_URL/packages/gcca.tgz" "$IMAGE_DIR/gcca.tgz" 554454
  download "$MCC_URL/packages/gccb.tgz" "$IMAGE_DIR/gccb.tgz" 825727
  download "$SVGA_URL" "$IMAGE_DIR/svgalib111.tar.gz" 305979

  for disk in nocdboot root; do
    if [[ ! -s "$IMAGE_DIR/$disk" ]]; then
      gzip -dc "$IMAGE_DIR/$disk.gz" > "$IMAGE_DIR/$disk"
    fi
    local actual
    actual="$(wc -c < "$IMAGE_DIR/$disk" | tr -d ' ')"
    [[ "$actual" == 1228800 ]] || die "$disk image should be 1,228,800 bytes, got $actual"
  done

  echo "==> Image checksums (recorded locally for reproducibility)"
  (cd "$IMAGE_DIR" && sha256sum nocdboot root gcca.tgz gccb.tgz svgalib111.tar.gz)

  echo "==> Creating compiler and svgalib floppy images"
  "$PIL" --new-tar-floppy "$IMAGE_DIR/gcca.flp" "$IMAGE_DIR/gcca.tgz" programs/linux/mandel.c
  "$PIL" --new-tar-floppy "$IMAGE_DIR/gccb.flp" "$IMAGE_DIR/gccb.tgz"
  python3 tools/mcc/ustar_repack.py "$IMAGE_DIR/svgalib111.tar.gz" "$IMAGE_DIR/svga.tgz"
  "$PIL" --new-tar-floppy "$IMAGE_DIR/svga.flp" "$IMAGE_DIR/svga.tgz" tools/mcc/svgabld.sh programs/linux/vgamand.c

  if [[ ! -e "$IMAGE_DIR/hd40.img" ]]; then
    echo "==> Creating fresh 40 MB MCC hard disk"
    "$PIL" --new-hd-image "$IMAGE_DIR/hd40.img" 40
    DISK_WAS_NEW=1
  else
    echo "    Keeping existing persistent disk: $IMAGE_DIR/hd40.img"
    DISK_WAS_NEW=0
  fi
}

run_probe() {
  local script="$1" steps="$2" extra=("${@:3}")
  "$PIL" --boot-probe "$IMAGE_DIR/nocdboot" --hd-image "$IMAGE_DIR/hd40.img" \
    --script "$script" --steps "$steps" --rtc-year 1994 "${extra[@]}"
}

if (( DO_PREPARE )); then
  prepare_media
else
  [[ -x "$PIL" ]] || die "PIL build missing"
  [[ -s "$IMAGE_DIR/nocdboot" && -s "$IMAGE_DIR/hd40.img" ]] || die "Run --prepare first"
  DISK_WAS_NEW=0
fi

if (( DO_INSTALL )); then
  if [[ "${DISK_WAS_NEW:-0}" != 1 ]]; then
    if (( EXPLICIT_STAGE )); then
      die "--install only runs on a newly-created disk; move the existing hd40.img aside first"
    else
      echo "==> Existing hd40.img detected: skipping destructive MCC Base installation"
    fi
  else
    echo "==> Installing MCC Interim Base (ext2, no swap/LILO/packages)"
    run_probe tools/mcc/install.txt 4000000000
  fi
fi

if (( DO_GCC )); then
  [[ -s "$IMAGE_DIR/hd40.img" ]] || die "Missing hard disk; run --prepare and --install first"
  echo "==> Installing gcc 2.5.8 (gcca + gccb)"
  run_probe tools/mcc/gcc.txt 4000000000
fi

if (( DO_MANDEL )); then
  echo "==> Running fixed-point and 80387 Mandelbrot benchmarks"
  run_probe tools/mcc/mandel.txt 9000000000 --fpu 80387
fi

if (( DO_SVGALIB )); then
  echo "==> Building svgalib 1.1 and running VGA/Mandelbrot tests"
  run_probe tools/mcc/svgalib.txt 4000000000 --fpu 80387
  run_probe tools/mcc/vgatest.txt 3000000000 --fpu 80387
  run_probe tools/mcc/vgamand.txt 3000000000 --fpu 80387
fi

echo
echo "==> MCC 386 environment workflow finished"
echo "    Disk: $IMAGE_DIR/hd40.img"
echo "    Floppies: $IMAGE_DIR/*.flp"
echo "    Logs/screenshots are reported by the existing MCC probe scripts."
