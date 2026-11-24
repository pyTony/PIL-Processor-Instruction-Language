#!/usr/bin/env bash
# Build and exercise MCC Interim 1.0+ (Linux 1.0.4) as a 386 guest in PIL/Dine.
# Intended for Linux and Termux; reuses the checked-in tools/mcc guest scripts.
#
#   bash tools/mcc/build-termux.sh [--all|--prepare|--install|--gcc|--mandel|--svgalib]
#
# --all is the default. Existing disk images are preserved. --all skips the
# installer when hd40.img already exists; --install refuses to overwrite it.
# To reinstall, move images/mcc-1.0/hd40.img aside before running --install.

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
INSTALL_EXPLICIT=0
SELECTED=0

usage() {
  cat <<'EOF'
Usage: bash tools/mcc/build-termux.sh [stage]

  --all       Prepare, install MCC Base if needed, then run gcc, 80387 and
              svgalib builds/tests (default).
  --prepare   Build PIL and download/prepare all MCC and compiler media.
  --install   Install MCC Base onto a NEW 40 MB disk image.
  --gcc       Install gcc 2.5.8 packages onto an installed MCC disk.
  --mandel    Compile and time fixed-point and 80387 Mandelbrot tests.
  --svgalib   Build svgalib 1.1 and run its VGA tests.
  -h, --help  Show this help.

All stages prepare the downloads and floppy images first. The persistent
images/mcc-1.0/hd40.img is never silently replaced.
EOF
}

for arg in "$@"; do
  case "$arg" in
    --all) DO_INSTALL=1; DO_GCC=1; DO_MANDEL=1; DO_SVGALIB=1; SELECTED=1 ;;
    --prepare) DO_PREPARE=1; SELECTED=1 ;;
    --install) DO_INSTALL=1; INSTALL_EXPLICIT=1; SELECTED=1 ;;
    --gcc) DO_GCC=1; SELECTED=1 ;;
    --mandel) DO_MANDEL=1; SELECTED=1 ;;
    --svgalib) DO_SVGALIB=1; SELECTED=1 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $arg" >&2; usage >&2; exit 2 ;;
  esac
done

if (( SELECTED == 0 )); then
  DO_INSTALL=1; DO_GCC=1; DO_MANDEL=1; DO_SVGALIB=1
fi
DO_PREPARE=1

die() { echo "ERROR: $*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "Missing '$1'. On Termux install it with: pkg install $2"; }

need cargo rust
need curl curl
need python3 python
need gzip gzip
need sha256sum coreutils

echo "==> Repository: $ROOT"
echo "==> Host: $(uname -srm)"
echo "==> Building PIL/Dine (release, locked; same build mode as PRs #307-#310)"
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

create_floppy() {
  local output="$1"
  shift
  if [[ -s "$output" ]]; then
    echo "    Keeping existing $(basename "$output")"
  else
    "$PIL" --new-tar-floppy "$output" "$@"
  fi
}

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
  actual="$(wc -c < "$IMAGE_DIR/$disk" | tr -d ' ')"
  [[ "$actual" == 1228800 ]] || die "$disk image should be 1,228,800 bytes, got $actual"
done

echo "==> SHA-256 values"
(cd "$IMAGE_DIR" && sha256sum nocdboot root gcca.tgz gccb.tgz svgalib111.tar.gz)

echo "==> Preparing compiler and svgalib floppy images"
create_floppy "$IMAGE_DIR/gcca.flp" "$IMAGE_DIR/gcca.tgz" programs/linux/mandel.c
create_floppy "$IMAGE_DIR/gccb.flp" "$IMAGE_DIR/gccb.tgz"
if [[ ! -s "$IMAGE_DIR/svga.tgz" ]]; then
  python3 tools/mcc/ustar_repack.py "$IMAGE_DIR/svgalib111.tar.gz" "$IMAGE_DIR/svga.tgz"
fi
create_floppy "$IMAGE_DIR/svga.flp" "$IMAGE_DIR/svga.tgz" tools/mcc/svgabld.sh programs/linux/vgamand.c

run_probe() {
  local script="$1"
  local steps="$2"
  shift 2
  "$PIL" --boot-probe "$IMAGE_DIR/nocdboot" --hd-image "$IMAGE_DIR/hd40.img" \
    --script "$script" --steps "$steps" --rtc-year 1994 "$@"
}

if (( DO_INSTALL )); then
  if [[ -e "$IMAGE_DIR/hd40.img" ]]; then
    if (( INSTALL_EXPLICIT )); then
      die "--install will not overwrite existing hd40.img; move it aside first"
    fi
    echo "==> Existing hd40.img found; skipping destructive MCC Base installation"
  else
    echo "==> Creating a new 40 MB hard disk"
    "$PIL" --new-hd-image "$IMAGE_DIR/hd40.img" 40
    echo "==> Installing MCC Interim Base (ext2, no swap/LILO/packages)"
    run_probe tools/mcc/install.txt 4000000000
  fi
fi

if (( DO_GCC || DO_MANDEL || DO_SVGALIB )); then
  [[ -s "$IMAGE_DIR/hd40.img" ]] || die "No installed MCC hard disk. Run --install first."
fi

if (( DO_GCC )); then
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
echo "    Persistent disk: $IMAGE_DIR/hd40.img"
echo "    Floppy images:   $IMAGE_DIR/*.flp"
