#!/usr/bin/env python3
"""Tk text-mode screen for a PIL DOS session (Python 3.10+).

Runs `pil-proof --dos-session PROGRAM`, draws the B800 text page it reports
and sends key presses as BIOS keystrokes. Execution, DOS services and the
screen contents all come from the engine; this file only displays them.

    python tools/dos_terminal.py images/FD14LITE.img::COMMAND.COM
    python tools/dos_terminal.py --mount dosfiles images/FD14LITE.img::COMMAND.COM

An image becomes read-only drive C:; --mount adds a host folder as the next
drive (D: after an image), also read-only.
"""
import argparse
import json
from pathlib import Path
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
from pil_debugger import find_engine  # noqa: E402

MAX_REPLY = 65536
SLICE = 20000
# CGA palette for the 16 text attributes.
PALETTE = ['#000000', '#0000aa', '#00aa00', '#00aaaa', '#aa0000', '#aa00aa', '#aa5500', '#aaaaaa',
           '#555555', '#5555ff', '#55ff55', '#55ffff', '#ff5555', '#ff55ff', '#ffff55', '#ffffff']
# US-layout set 1 scan codes for the keys a DOS program sees as ASCII.
_ROWS = [('1234567890-=', 0x02), ('qwertyuiop[]', 0x10), ("asdfghjkl;'`", 0x1e), ('zxcvbnm,./', 0x2c)]
_SHIFTED = {'!': '1', '@': '2', '#': '3', '$': '4', '%': '5', '^': '6', '&': '7', '*': '8', '(': '9', ')': '0',
            '_': '-', '+': '=', '{': '[', '}': ']', ':': ';', '"': "'", '~': '`', '<': ',', '>': '.', '?': '/', '|': '\\'}
SCAN = {c: start + i for row, start in _ROWS for i, c in enumerate(row)}
SCAN.update({'\\': 0x2b, ' ': 0x39})
# Keys with no ASCII value: (ascii, scan) as the BIOS stores them.
SPECIAL = {'Return': (13, 0x1c), 'KP_Enter': (13, 0x1c), 'BackSpace': (8, 0x0e), 'Tab': (9, 0x0f),
           'Escape': (27, 0x01), 'Up': (0, 0x48), 'Down': (0, 0x50), 'Left': (0, 0x4b), 'Right': (0, 0x4d),
           'Home': (0, 0x47), 'End': (0, 0x4f), 'Prior': (0, 0x49), 'Next': (0, 0x51),
           'Insert': (0, 0x52), 'Delete': (0, 0x53)}
SPECIAL.update({f'F{n}': (0, 0x3a + n) for n in range(1, 11)})
# Letters beyond ASCII take the scan code of the key they sit on in the Nordic
# layouts (Finnish/Swedish: å ä ö; Danish/Norwegian: å æ ø); others get 0.
_NATIONAL = {'å': 0x1a, 'ä': 0x28, 'ö': 0x27, 'æ': 0x28, 'ø': 0x27, '§': 0x29, '½': 0x29}


def key_code(keysym, char, control=False):
    """Map a Tk key event to (ascii, scan), or None for keys DOS never sees."""
    if keysym in SPECIAL:
        return SPECIAL[keysym]
    if not char or len(char) != 1:
        return None
    if ord(char) > 0x7e:
        # The host keyboard layout already chose the character (no KEYB needed):
        # pass it on in code page 437, or drop it if 437 has no such character.
        try:
            code = char.encode('cp437')[0]
        except UnicodeEncodeError:
            return None
        return code, _NATIONAL.get(char.lower(), 0)
    if 1 <= ord(char) <= 26:  # Tk already turned Ctrl+letter into a control code
        return ord(char), SCAN[chr(ord(char) + 0x60)]
    base = char.lower() if char.isalpha() else _SHIFTED.get(char, char)
    scan = SCAN.get(base)
    if control and char.isalpha():
        return ord(char.lower()) - 0x60, scan
    if scan is None and ord(char) >= 0x20:
        return None
    return ord(char), scan or 0


def screen_rows(reply):
    """Rows of (text, attributes) from a session reply's character/attribute bytes."""
    cells = bytes.fromhex(reply['screen'])
    columns = reply['columns']
    rows = []
    for r in range(len(cells) // (2 * columns)):
        row = cells[r * 2 * columns:(r + 1) * 2 * columns]
        rows.append((row[0::2].decode('cp437'), list(row[1::2])))
    return rows


class Session:
    """The engine process: one command line out, one JSON reply line back."""

    def __init__(self, engine, program, mount=None):
        mount_args = ['--mount', str(mount)] if mount else []
        self.process = subprocess.Popen([str(engine), '--dos-session', *mount_args, str(program)], stdin=subprocess.PIPE,
                                        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding='utf-8',
                                        creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
        self.seq = 0
        self.last = self._read()

    def _read(self):
        line = self.process.stdout.readline(MAX_REPLY)
        if not line:
            raise RuntimeError(self.process.stderr.read().strip() or 'PIL DOS session ended')
        reply = json.loads(line)
        if reply.get('protocol') != 1 or reply.get('seq') != self.seq:
            raise RuntimeError('unexpected reply from PIL DOS session')
        return reply

    def send(self, command):
        self.seq += 1
        self.process.stdin.write(command + '\n')
        self.process.stdin.flush()
        self.last = self._read()
        return self.last

    def close(self):
        if self.process.poll() is None:
            try:
                self.send('quit')
            except (OSError, RuntimeError, ValueError):
                pass
            self.process.kill()


def make_app(root, engine, program, mount=None):
    import tkinter as tk
    from tkinter import font as tkfont

    root.title(f'PIL DOS — {program}')
    face = tkfont.Font(family='Courier New' if sys.platform == 'win32' else 'Courier', size=14)
    text = tk.Text(root, width=80, height=25, font=face, bg=PALETTE[0], fg=PALETTE[7], bd=0,
                   highlightthickness=0, cursor='arrow', wrap='none')
    text.pack(padx=8, pady=(8, 4))
    status = tk.Label(root, anchor='w', font=('TkDefaultFont', 10))
    status.pack(fill='x', padx=8, pady=(0, 6))
    for attr in range(256):
        text.tag_configure(f'a{attr}', foreground=PALETTE[attr & 15], background=PALETTE[(attr >> 4) & 7])
    text.tag_configure('cursor', underline=True)
    session = Session(engine, program, mount)
    keys = []

    def draw(reply):
        text.configure(state='normal')
        text.delete('1.0', 'end')
        for r, (chars, attrs) in enumerate(screen_rows(reply)):
            start = 0
            for c in range(1, len(chars) + 1):
                if c == len(chars) or attrs[c] != attrs[start]:
                    text.insert('end', chars[start:c], f'a{attrs[start]}')
                    start = c
            if r < 24:
                text.insert('end', '\n')
        row, column = reply['cursor']
        text.tag_add('cursor', f'{row + 1}.{column}')
        text.configure(state='disabled')
        detail = f" — {reply['detail']}" if reply['detail'] else ''
        status.configure(text=f"{reply['status']}{detail}   steps {reply['steps']}   CS:IP {reply['cs']:04x}:{reply['eip']:04x}")

    def on_key(event):
        code = key_code(event.keysym, event.char, bool(event.state & 0x4))
        if code:
            keys.append(code)
        return 'break'

    def tick():
        try:
            reply = session.last
            while keys and reply['status'] in ('running', 'waiting_input'):
                ascii_code, scan = keys[0]
                reply = session.send(f'key {ascii_code} {scan}')
                if not reply['ok']:
                    break
                keys.pop(0)
            if reply['status'] in ('running', 'waiting_input'):
                reply = session.send(f'run {SLICE}')
            draw(reply)
        except (OSError, RuntimeError, ValueError) as error:
            status.configure(text=f'session error: {error}')
            return
        root.after(15 if session.last['status'] == 'running' else 40, tick)

    def close():
        session.close()
        root.destroy()

    root.bind('<Key>', on_key)
    root.protocol('WM_DELETE_WINDOW', close)
    draw(session.last)
    root.after(10, tick)
    return session


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('program', help='DOS .COM/.EXE, or IMAGE.img::INNER_PATH')
    parser.add_argument('--mount', help='host folder shown as the next read-only drive (D: after an image)')
    parser.add_argument('--engine', help='pil-proof executable (default: newest build)')
    args = parser.parse_args(argv)
    import tkinter as tk
    root = tk.Tk()
    make_app(root, find_engine(args.engine), args.program, args.mount)
    root.mainloop()


if __name__ == '__main__':
    main()
