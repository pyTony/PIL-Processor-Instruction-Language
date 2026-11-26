#!/usr/bin/env python3
"""Check the X11 window of `pil --dos-console --window` against a virtual X server.

Needs a pil built with `--features x11`, Xvfb and python-xlib (pip install
python-xlib). Without them it says so and exits 0, so it can sit in a list of
checks. It is a by-hand check (CI does not install an X server).

What it does: writes an original boot sector that polls the keyboard
controller (port 60h) with interrupts off and prints each byte it reads as two
hex digits, boots it in the window, presses keys through the XTEST extension
and reads the screen back from the window, recognising the digits against
Dine's own font (src/bios/font8x16.txt). So it checks what a booted machine
such as Linux receives: set-1 scan codes with Shift, Ctrl, E0h keys, F-keys
and a held key's repeats. Then the window is closed through the window
manager protocol and pil must exit.

    python tools/verify_x11_window.py [--pil PATH]
"""
import argparse
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# X key codes are evdev codes + 8.
A, SHIFT_L, CTRL_L, C, UP, F1, DELETE, CTRL_R, KP_ENTER = 38, 50, 37, 54, 111, 67, 119, 105, 104
CASES = [
    ("a", [A], "1E 9E"),
    ("Shift+a", [SHIFT_L, A], "2A 1E 9E AA"),
    ("Up", [UP], "E0 48 E0 C8"),
    ("Ctrl+c", [CTRL_L, C], "1D 2E AE 9D"),
    ("F1", [F1], "3B BB"),
    ("Delete", [DELETE], "E0 53 E0 D3"),
    ("right Ctrl", [CTRL_R], "E0 1D E0 9D"),
    ("keypad Enter", [KP_ENTER], "E0 1C E0 9C"),
]


def boot_image():
    """A 1.44 MB diskette whose boot sector prints every byte of port 60h."""
    code = bytearray()

    def emit(*b):
        code.extend(b)

    emit(0xFA)                                  # cli: poll, do not take IRQ1
    emit(0x31, 0xC0, 0x8E, 0xD8)                # xor ax,ax ; mov ds,ax
    vec = len(code)
    emit(0xC7, 0x06, 0x24, 0x00, 0, 0)          # INT 09h offset: a guest handler,
    emit(0xC7, 0x06, 0x26, 0x00, 0, 0)          # so keys come through port 60h
    loop = len(code)
    emit(0xE4, 0x64, 0xA8, 0x01, 0x74, 0xFA)    # in al,64h ; test al,1 ; jz loop
    emit(0xE4, 0x60, 0x88, 0xC3)                # in al,60h ; mov bl,al
    emit(0xC0, 0xE8, 0x04)                      # shr al,4
    call1 = len(code)
    emit(0xE8, 0, 0)                            # call hex
    emit(0x88, 0xD8)                            # mov al,bl
    call2 = len(code)
    emit(0xE8, 0, 0)                            # call hex
    emit(0xB0, 0x20, 0xB4, 0x0E, 0xCD, 0x10)    # a space, BIOS teletype
    jmp = len(code)
    emit(0xEB, 0)                               # jmp loop
    hexfn = len(code)
    emit(0x24, 0x0F, 0x04, 0x30, 0x3C, 0x39, 0x76, 0x02, 0x04, 0x07,
         0xB4, 0x0E, 0xCD, 0x10, 0xC3)         # and,add,cmp,jbe,add,teletype,ret
    for at, target in ((call1, hexfn), (call2, hexfn)):
        d = target - (at + 3)
        code[at + 1], code[at + 2] = d & 0xFF, (d >> 8) & 0xFF
    code[jmp + 1] = (loop - (jmp + 2)) & 0xFF
    handler = len(code)
    emit(0xCF)                                  # iret
    code[vec + 4], code[vec + 5] = (0x7C00 + handler) & 0xFF, (0x7C00 + handler) >> 8
    sector = bytes(code) + bytes(510 - len(code)) + b"\x55\xAA"
    return sector + bytes(1474560 - 512)


def font_templates():
    """Bitmaps of '0'-'9', 'A'-'F' and the space from the project's 8x16 font."""
    lines = open(os.path.join(ROOT, "src", "bios", "font8x16.txt"), encoding="utf-8").read().splitlines()
    wanted = {ord(c): c for c in "0123456789ABCDEF "}
    found, i = {}, 0
    while i < len(lines):
        m = re.match(r"^([0-9A-F]{2})(?: .*)?$", lines[i])
        if m and i + 16 < len(lines) and re.match(r"^[.#]{8}$", lines[i + 1]):
            code = int(m.group(1), 16)
            if code in wanted:
                found[tuple(tuple(c == "#" for c in row) for row in lines[i + 1:i + 17])] = wanted[code]
            i += 17
        else:
            i += 1
    return found


def read_text(window, templates, rows=4):
    """The first text rows of the window, one string, trailing blanks cut."""
    from Xlib import X
    geo = window.get_geometry()
    img = window.get_image(0, 0, geo.width, geo.height, X.ZPixmap, 0xFFFFFFFF)
    data, stride = img.data, geo.width * 4
    out = []
    for row in range(rows):
        text = ""
        for col in range(80):
            x0, y0 = col * 9, row * 16
            cell = tuple(tuple(data[(y0 + y) * stride + (x0 + x) * 4 + 2] > 0x55 for x in range(8)) for y in range(16))
            text += templates.get(cell, "?")
        out.append(text)
    return "".join(out).replace(" ", "").replace("?", "")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--pil", default=os.path.join(ROOT, "target", "release", "pil"), help="pil built with --features x11")
    args = ap.parse_args()
    if not shutil.which("Xvfb"):
        print("SKIP: Xvfb is not installed")
        return 0
    try:
        from Xlib import X, display, protocol
        from Xlib.ext import xtest
    except ImportError:
        print("SKIP: python-xlib is not installed (pip install python-xlib)")
        return 0
    if not os.path.exists(args.pil):
        print(f"SKIP: {args.pil} does not exist (cargo build --release --features x11)")
        return 0
    templates = font_templates()
    assert len(templates) == 17, "the font file did not give the 17 glyphs needed"

    with tempfile.TemporaryDirectory() as tmp:
        image = os.path.join(tmp, "keys.img")
        open(image, "wb").write(boot_image())
        xvfb = subprocess.Popen(["Xvfb", "-displayfd", "1", "-screen", "0", "1280x800x24", "-nolisten", "tcp"],
                                stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        pil = None
        try:
            name = ":" + xvfb.stdout.readline().strip()
            env = dict(os.environ, DISPLAY=name, DINE_WINDOW_SCALE="1")
            pil = subprocess.Popen([args.pil, "--dos-console", "--window", "--boot", image], env=env,
                                   stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
            d = display.Display(name)
            window = None
            for _ in range(100):
                for child in d.screen().root.query_tree().children:
                    if (child.get_wm_name() or "").startswith("Dine"):
                        window = child
                if window or pil.poll() is not None:
                    break
                time.sleep(0.1)
            if window is None:
                print("FAIL: no window appeared;", (pil.stdout.read() if pil.poll() is not None else "pil still running"))
                return 1
            time.sleep(1.0)

            def press(codes, hold=0.0):
                for c in codes:
                    xtest.fake_input(d, X.KeyPress, c)
                d.sync()
                time.sleep(hold or 0.05)
                for c in reversed(codes):
                    xtest.fake_input(d, X.KeyRelease, c)
                d.sync()
                time.sleep(0.15)

            failures = 0
            expected = ""
            for label, codes, scan in CASES:
                press(codes)
                expected += scan.replace(" ", "")
            time.sleep(0.5)
            got = read_text(window, templates)
            ok = got == expected
            print(f"{'ok  ' if ok else 'FAIL'} scan codes: {', '.join(c[0] for c in CASES)}")
            if not ok:
                print(f"     expected {expected}\n     got      {got}")
                failures += 1

            before = len(got)
            press([A], hold=1.0)
            time.sleep(0.5)
            tail = read_text(window, templates)[before:]
            ok = re.fullmatch(r"(1E){3,}9E", tail) is not None
            print(f"{'ok  ' if ok else 'FAIL'} a held key repeats its make code and breaks once: {tail}")
            failures += 0 if ok else 1

            message = protocol.event.ClientMessage(window=window, client_type=d.intern_atom("WM_PROTOCOLS"),
                                                   data=(32, [d.intern_atom("WM_DELETE_WINDOW"), X.CurrentTime, 0, 0, 0]))
            window.send_event(message)
            d.sync()
            try:
                pil.wait(timeout=5)
                ok = pil.returncode == 0
            except subprocess.TimeoutExpired:
                ok = False
            print(f"{'ok  ' if ok else 'FAIL'} closing the window ends pil")
            failures += 0 if ok else 1
            return 1 if failures else 0
        finally:
            if pil is not None and pil.poll() is None:
                pil.kill()
            xvfb.terminate()


if __name__ == "__main__":
    sys.exit(main())
