#!/usr/bin/env python3
"""Tk text-mode screen for a PIL DOS session (Python 3.10+).

Runs `pil --dos-session PROGRAM`, draws the B800 text page it reports
and sends key presses as BIOS keystrokes. Execution, DOS services and the
screen contents all come from the engine; this file only displays them.

    python tools/dos_terminal.py images/FD14LITE.img::COMMAND.COM
    python tools/dos_terminal.py --mount dosfiles images/FD14LITE.img::COMMAND.COM
    python tools/dos_terminal.py --mount dosfiles --args "/K D:\\START.BAT" images/FD14LITE.img::COMMAND.COM

An image becomes read-only drive C:; --mount adds a host folder as the next
drive (D: after an image), which DOS programs can write: files they create,
change, delete or rename change in the host folder itself.
--com1 console connects COM1 to the console this was started from (CTTY COM1
runs the shell there), --com1 tcp:2323 to a TCP port for telnet or PuTTY.
--country 358 gives Finnish dates and times (31.10.2026, 14.27.05).
--args gives the program its command line; for COMMAND.COM, /K runs a batch
file or command and then stays at the prompt.
Z: is a drive held in memory for the session, with TEMP=Z:\\ so that pipes
work from any drive; what is written there is gone when the session ends.

DOS date and time follow this computer's clock in its time zone, or in the
one --utc-offset gives (hours, such as 2 or -5:30); the status line shows it. Ctrl+Shift+C copies the
selection (or the whole screen), Ctrl+Shift+V or Shift+Insert types the
clipboard; both are also on the right-click menu. Shift, Ctrl and Alt reach
the BIOS shift flags, and Alt with a letter, digit or F-key sends its Alt code,
so F10 or Alt+F opens a menu in EDIT.
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
# Slices a program runs after a key before it sees a shift key change.
SETTLE = 10
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
# The 256 glyphs of the IBM PC character ROM: code page 437 with the symbols
# the hardware draws for 01h-1Fh and 7Fh, where Python's codec has controls.
GLYPHS = (' ☺☻♥♦♣♠•◘○◙♂♀♪♫☼►◄↕‼¶§▬↨↑↓→←∟↔▲▼'
          + bytes(range(0x20, 0x7f)).decode('ascii') + '⌂'
          + bytes(range(0x80, 0x100)).decode('cp437'))
# Characters a host keyboard can type, back to their byte (NUL is never typed).
TYPED = {c: n for n, c in enumerate(GLYPHS) if n >= 0x80 or n in range(0x14, 0x16)}
# Letters beyond ASCII take the scan code of the key they sit on in the Nordic
# layouts (Finnish/Swedish: å ä ö; Danish/Norwegian: å æ ø); others get 0.
_NATIONAL = {'å': 0x1a, 'ä': 0x28, 'ö': 0x27, 'æ': 0x28, 'ø': 0x27, '§': 0x29, '½': 0x29}


def key_code(keysym, char, control=False, alt=False):
    """Map a Tk key event to (ascii, scan), or None for keys DOS never sees."""
    # AltGr (Ctrl+Alt on Windows) types characters such as \\ @ { } on
    # national layouts: only Alt with a letter, digit or F-key is an Alt code.
    if alt and alt_code(keysym):
        return alt_code(keysym)
    if keysym in SPECIAL:
        return SPECIAL[keysym]
    if not char or len(char) != 1:
        return None
    if ord(char) > 0x7e:
        # The host keyboard layout already chose the character (no KEYB needed):
        # pass it on in code page 437 (also § and ¶ at 15h/14h), or drop it.
        if char not in TYPED:
            return None
        return TYPED[char], _NATIONAL.get(char.lower(), 0)
    if 1 <= ord(char) <= 26:  # Tk already turned Ctrl+letter into a control code
        return ord(char), SCAN[chr(ord(char) + 0x60)]
    base = char.lower() if char.isalpha() else _SHIFTED.get(char, char)
    scan = SCAN.get(base)
    if control and char.isalpha():
        return ord(char.lower()) - 0x60, scan
    if scan is None and ord(char) >= 0x20:
        return None
    return ord(char), scan or 0


def alt_code(keysym):
    """Alt with a letter, digit or F-key, as the BIOS stores it: ASCII 0 and
    the key's scan code (78h-83h for the top row's 1-0, -, =; 68h-71h for F1-F10)."""
    if len(keysym) == 1 and keysym.lower() in SCAN and keysym.lower().isalpha():
        return 0, SCAN[keysym.lower()]
    if len(keysym) == 1 and keysym in '1234567890-=':
        return 0, 0x78 + '1234567890-='.index(keysym)
    if keysym in SPECIAL and keysym.startswith('F'):
        return 0, SPECIAL[keysym][1] + 0x2d
    return None


# Modifier keys as their BIOS shift-flag bits at 0040:0017.
MODIFIERS = {'Shift_R': 0x01, 'Shift_L': 0x02, 'Control_L': 0x04, 'Control_R': 0x04, 'Alt_L': 0x08, 'Alt_R': 0x08}


def paste_keys(text):
    """Keystrokes that type `text`: line ends as Enter, tabs as Tab; characters
    code page 437 cannot hold are left out."""
    keys = []
    for char in text.replace('\r\n', '\n').replace('\r', '\n'):
        if char == '\n':
            keys.append(SPECIAL['Return'])
        elif char == '\t':
            keys.append((9, 0x0f))
        else:
            code = key_code(char, char)
            if code:
                keys.append(code)
    return keys


def key_command(code):
    return 'key {} {}'.format(*code)


def screen_rows(reply):
    """Rows of (text, attributes) from a session reply's character/attribute bytes."""
    cells = bytes.fromhex(reply['screen'])
    columns = reply['columns']
    rows = []
    for r in range(len(cells) // (2 * columns)):
        row = cells[r * 2 * columns:(r + 1) * 2 * columns]
        rows.append((''.join(GLYPHS[b] for b in row[0::2]), list(row[1::2])))
    return rows


class Session:
    """The engine process: one command line out, one JSON reply line back."""

    def __init__(self, engine, program, mount=None, args=None):
        options = (['--mount', str(mount)] if mount else []) + (['--args', args] if args else [])
        self.process = subprocess.Popen([str(engine), '--dos-session', *options, str(program)], stdin=subprocess.PIPE,
                                        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding='utf-8',
                                        creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
        self.seq = 0
        self.com1_out = b''  # what the program sent to COM1, not yet shown
        self.last = self._read()

    def _read(self):
        line = self.process.stdout.readline(MAX_REPLY)
        if not line:
            raise RuntimeError(self.process.stderr.read().strip() or 'PIL DOS session ended')
        reply = json.loads(line)
        if reply.get('protocol') != 1 or reply.get('seq') != self.seq:
            raise RuntimeError('unexpected reply from PIL DOS session')
        self.com1_out += bytes.fromhex(reply.get('com1', ''))
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


def local_offset():
    """Seconds east of UTC of local time now, daylight saving included."""
    from datetime import datetime
    return int(datetime.now().astimezone().utcoffset().total_seconds())


def parse_offset(text):
    """A UTC offset given as hours ("2", "-5", "5.5") or "+HH:MM", in seconds."""
    text = text.strip()
    sign = -1 if text.startswith('-') else 1
    body = text.lstrip('+-')
    if ':' in body:
        hours, minutes = body.split(':', 1)
        seconds = int(hours) * 3600 + int(minutes) * 60
    else:
        seconds = round(float(body) * 3600)
    if seconds > 14 * 3600:
        raise ValueError(f'UTC offset {text} is beyond 14 hours')
    return sign * seconds


# Keys a host console reports without a character, as the VT100 sequences a
# serial terminal sends (Windows msvcrt codes after 00h or E0h).
_VT_KEYS = {'H': b'\x1b[A', 'P': b'\x1b[B', 'M': b'\x1b[C', 'K': b'\x1b[D', 'G': b'\x1b[H', 'O': b'\x1b[F',
            'S': b'\x7f', 'R': b'\x1b[2~', 'I': b'\x1b[5~', 'Q': b'\x1b[6~'}


def to_cp437(text):
    """Host characters as the code page 437 bytes a DOS program reads; Enter is CR."""
    return text.replace('\r\n', '\r').replace('\n', '\r').encode('cp437', errors='ignore')


def from_cp437(data):
    """COM1 bytes for a host terminal: code page 437 text, control and escape
    bytes passed through, so ANSI sequences reach the terminal unchanged."""
    return data.decode('cp437')


class ConsolePort:
    """COM1 on the console this terminal was started from: what the program
    sends is printed there (an ANSI terminal draws its escape sequences), and
    keys typed there go to the program."""

    def __init__(self):
        import queue
        import threading
        if not sys.stdin or not sys.stdin.isatty():
            raise RuntimeError('--com1 console needs the terminal started from a console window')
        self.input = queue.Queue()
        if sys.platform == 'win32':
            self._enable_vt()
            reader = self._read_windows
        else:
            reader = self._read_posix
        threading.Thread(target=reader, daemon=True).start()

    @staticmethod
    def _enable_vt():
        import ctypes
        kernel = ctypes.windll.kernel32
        handle = kernel.GetStdHandle(-11)
        mode = ctypes.c_uint32()
        if kernel.GetConsoleMode(handle, ctypes.byref(mode)):
            kernel.SetConsoleMode(handle, mode.value | 0x0004)  # ENABLE_VIRTUAL_TERMINAL_PROCESSING

    def _read_windows(self):
        import msvcrt
        while True:
            char = msvcrt.getwch()
            if char in ('\x00', '\xe0'):
                self.input.put(_VT_KEYS.get(msvcrt.getwch(), b''))
            else:
                self.input.put(to_cp437(char))

    def _read_posix(self):
        import atexit
        import os
        import termios
        import tty
        fd = sys.stdin.fileno()
        saved = termios.tcgetattr(fd)
        atexit.register(termios.tcsetattr, fd, termios.TCSADRAIN, saved)
        tty.setraw(fd)
        while True:
            data = os.read(fd, 64)
            if not data:
                return
            self.input.put(to_cp437(data.decode('utf-8', errors='ignore')))

    def write(self, data):
        # In raw mode a bare LF only moves down; DOS sends CR LF itself.
        sys.stdout.write(from_cp437(data))
        sys.stdout.flush()

    def read(self):
        data = b''
        while not self.input.empty():
            data += self.input.get_nowait()
        return data


class TcpPort:
    """COM1 on a TCP port of this computer, for telnet or PuTTY (Raw): one
    client at a time; bytes pass both ways unchanged."""

    def __init__(self, port, host='127.0.0.1'):
        import socket
        self.server = socket.create_server((host, port))
        self.server.setblocking(False)
        self.client = None

    def _accept(self):
        if self.client is None:
            try:
                self.client, _ = self.server.accept()
                self.client.setblocking(False)
            except BlockingIOError:
                pass
        return self.client

    def write(self, data):
        client = self._accept()
        if client is not None:
            try:
                client.sendall(data)
            except OSError:
                self.client = None

    def read(self):
        client = self._accept()
        if client is None:
            return b''
        try:
            data = client.recv(4096)
        except BlockingIOError:
            return b''
        except OSError:
            data = b''
        if not data:
            self.client = None
        return data.replace(b'\r\n', b'\r').replace(b'\r\x00', b'\r')


def open_com1(spec):
    """`console`, or `tcp:PORT` (localhost) or `tcp:HOST:PORT`."""
    if spec == 'console':
        return ConsolePort()
    if spec.startswith('tcp:'):
        host, _, port = spec[4:].rpartition(':')
        return TcpPort(int(port), host or '127.0.0.1')
    raise ValueError(f'--com1 takes console or tcp:PORT, not {spec!r}')


def make_app(root, engine, program, mount=None, utc_offset=None, args=None, country=None, com1=None):
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
    # The colour tags paint every cell's background: keep the selection above
    # them, and visible when the window does not have the focus.
    text.configure(selectbackground='#3366cc', selectforeground='#ffffff', inactiveselectbackground='#3366cc')
    text.tag_raise('sel')
    session = Session(engine, program, mount, args)
    # DOS date and time come from this computer's clock in local time.
    offset = local_offset() if utc_offset is None else utc_offset
    session.send(f'clock {offset}')
    if country:
        session.send(f'country {country}')
    zone = f"UTC{'+' if offset >= 0 else '-'}{abs(offset) // 3600:02d}:{abs(offset) % 3600 // 60:02d}"
    keys = []
    held, state = set(), {'shift': 0, 'settle': 0}
    beeps = [session.last.get('beeps', 0)]

    drawn = [None]

    def draw(reply):
        shown = (reply['screen'], tuple(reply['cursor']))
        if shown != drawn[0]:
            drawn[0] = shown
            # Keep a selection being copied across the redraw.
            selection = text.tag_ranges('sel')
            text.configure(state='normal')
            text.delete('1.0', 'end')
            fill(reply)
            if selection:
                text.tag_add('sel', *selection)
            text.configure(state='disabled')
        detail = f" — {reply['detail']}" if reply['detail'] else ''
        status.configure(text=f"{reply['status']}{detail}   steps {reply['steps']}   CS:IP {reply['cs']:04x}:{reply['eip']:04x}   clock {zone}")
        # The program switched the PC speaker on: sound the host's bell.
        if reply.get('beeps', 0) > beeps[0]:
            beeps[0] = reply['beeps']
            root.bell()

    def fill(reply):
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

    def copy(event=None):
        """The selection, or the whole screen without trailing blanks."""
        if text.tag_ranges('sel'):
            chosen = text.get('sel.first', 'sel.last')
        else:
            chosen = '\n'.join(line.rstrip() for line in text.get('1.0', 'end-1c').split('\n')).rstrip('\n')
        root.clipboard_clear()
        root.clipboard_append(chosen)
        return 'break'

    def paste(event=None):
        try:
            keys.extend(map(key_command, paste_keys(root.clipboard_get())))
        except tk.TclError:
            pass  # nothing text-like on the clipboard
        return 'break'

    def on_key(event):
        # Ctrl+Shift+C and Ctrl+Shift+V (and Shift+Insert) copy and paste; plain
        # Ctrl+C and Ctrl+V still go to DOS.
        control, shift = bool(event.state & 0x4), bool(event.state & 0x1)
        if control and shift and event.keysym.lower() in ('c', 'v'):
            return copy() if event.keysym.lower() == 'c' else paste()
        if shift and event.keysym == 'Insert':
            return paste()
        if event.keysym in MODIFIERS:
            held.add(event.keysym)
            return shift_changed()
        # Alt_R with Ctrl is AltGr on Windows: it types the character it makes.
        alt = 'Alt_L' in held or ('Alt_R' in held and not control)
        code = key_code(event.keysym, event.char, control, alt)
        if code:
            keys.append(key_command(code))
        return 'break'

    def on_release(event):
        held.discard(event.keysym)
        return shift_changed()

    def shift_changed():
        flags = 0
        for name in held:
            flags |= MODIFIERS[name]
        if flags != state['shift']:
            state['shift'] = flags
            keys.append(f'shift {flags}')
        return 'break'

    def lost_focus(event):
        held.clear()
        shift_changed()

    def tick():
        try:
            reply = session.last
            sent = False
            state['settle'] = max(0, state['settle'] - 1)
            while keys and reply['status'] in ('running', 'waiting_input'):
                # A change of the shift keys waits until the program has run a
                # while after the keys before it, so that EDIT reads Alt+F
                # before it sees Alt go up, as it would at a person's pace.
                if keys[0].startswith('shift') and (sent or state['settle']):
                    break
                reply = session.send(keys[0])
                if not reply['ok']:
                    break
                sent = keys.pop(0)
                if sent.startswith('shift'):
                    break
                state['settle'] = SETTLE
            if com1:
                incoming = com1.read()
                for at in range(0, len(incoming), 100):
                    if reply['status'] in ('running', 'waiting_input'):
                        reply = session.send('com1 ' + incoming[at:at + 100].hex())
            if reply['status'] in ('running', 'waiting_input'):
                reply = session.send(f'run {SLICE}')
            if com1 and session.com1_out:
                com1.write(session.com1_out)
                session.com1_out = b''
            draw(reply)
        except (OSError, RuntimeError, ValueError) as error:
            status.configure(text=f'session error: {error}')
            return
        root.after(15 if session.last['status'] == 'running' else 40, tick)

    def close():
        session.close()
        root.destroy()

    root.bind('<Key>', on_key)
    root.bind('<KeyRelease>', on_release)
    root.bind('<FocusOut>', lost_focus)
    menu = tk.Menu(root, tearoff=0)
    menu.add_command(label='Copy (Ctrl+Shift+C)', command=copy)
    menu.add_command(label='Paste (Ctrl+Shift+V)', command=paste)
    text.bind('<Button-3>', lambda event: menu.tk_popup(event.x_root, event.y_root))
    root.protocol('WM_DELETE_WINDOW', close)
    draw(session.last)
    root.after(10, tick)
    return session


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('program', help='DOS .COM/.EXE, or IMAGE.img::INNER_PATH')
    parser.add_argument('--mount', help='host folder shown as the next drive (D: after an image); DOS writes reach it')
    parser.add_argument('--args', metavar='COMMAND_LINE',
                        help='command line for the program, e.g. "/K D:\\START.BAT" to have COMMAND.COM run a batch file first')
    parser.add_argument('--country', type=int, choices=[1, 358],
                        help='DOS country: 1 (USA, the default: 10-31-2026, 2:27p) or 358 (Finland: 31.10.2026, 14.27)')
    parser.add_argument('--com1', metavar='console|tcp:PORT',
                        help='connect COM1 to the console this was started from, or to a TCP port for telnet/PuTTY')
    parser.add_argument('--engine', help='pil executable (default: newest build)')
    parser.add_argument('--utc-offset', type=parse_offset, metavar='HOURS',
                        help='time zone of the DOS clock, e.g. 2, -5, 5.5 or +02:00 (default: this computer\'s)')
    args = parser.parse_args(argv)
    com1 = open_com1(args.com1) if args.com1 else None
    import tkinter as tk
    root = tk.Tk()
    make_app(root, find_engine(args.engine), args.program, args.mount, args.utc_offset, args.args, args.country, com1)
    root.mainloop()


if __name__ == '__main__':
    main()
