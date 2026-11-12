#!/usr/bin/env python3
"""Tk screen for a PIL DOS session (Python 3.10+).

Runs `pil --dos-session PROGRAM`, draws the B800 text page it reports
and sends key presses as BIOS keystrokes. Execution, DOS services and the
screen contents all come from the engine; this file only displays them.
CGA modes 04h-06h, EGA/VGA 16-colour modes 0Dh/0Eh/10h/12h and VGA mode
13h are displayed at 4:3, like a CRT;
text mode uses the host font.

    python tools/dos_terminal.py images/FD14LITE.img::COMMAND.COM
    python tools/dos_terminal.py --mount dosfiles images/FD14LITE.img::COMMAND.COM
    python tools/dos_terminal.py --mount dosfiles --args "/K D:\\START.BAT" images/FD14LITE.img::COMMAND.COM

An image becomes read-only drive C:; --mount adds a host folder as the next
drive (D: after an image), which DOS programs can write: files they create,
change, delete or rename change in the host folder itself.
--com1 console connects COM1 to the console this was started from (CTTY COM1
runs the shell there), --com1 tcp:2323 to a TCP port for telnet or PuTTY.
--country 358 gives Finnish dates and times (31.10.2026, 14.27.05).
--speaker-volume 0 mutes PC speaker clicks; 1..100 adjusts their volume (default 5).
--ansi loads an ANSI.SYS-style console driver, as DEVICE=ANSI.SYS would: then
PROMPT $e[1;36m colours and other ESC [ sequences draw on the DOS screen.
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
import codecs
import io
import json
import math
import os
import shutil
import struct
import threading
import time
import wave
from pathlib import Path
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
from pil_debugger import find_engine  # noqa: E402


def find_dos_engine(explicit=None):
    """Use an ordinary Cargo build, or the engine explicitly requested."""
    return find_engine(explicit)

MAX_REPLY = 8 * 1024 * 1024  # VGA frame plus escaped DOS and serial output
SLICE = 20000
# A tick keeps running slices for up to this many seconds before it redraws,
# so that a busy program is not held up by drawing a frame every slice.
FRAME = 1 / 120
# Steps in each further slice of a tick, once the first has shown the
# program is still busy.
BURST = 100000
# Slices a program runs after a key before it sees a shift key change.
SETTLE = 10
# CGA palette for the 16 text attributes.
PALETTE = ['#000000', '#0000aa', '#00aa00', '#00aaaa', '#aa0000', '#aa00aa', '#aa5500', '#aaaaaa',
           '#555555', '#5555ff', '#55ff55', '#55ffff', '#ff5555', '#ff55ff', '#ffff55', '#ffffff']
# Host PC speaker: the DOS session reports activation counts, not PCM or the
# duration of each pulse. A short, quiet click is therefore an approximation.
# It avoids playing a long system notification for each Landmark speaker pulse.
SPEAKER_INTERVAL = 0.2


def speaker_volume(value):
    try:
        level = int(value)
    except ValueError:
        raise argparse.ArgumentTypeError('speaker volume must be 0..100')
    if not 0 <= level <= 100:
        raise argparse.ArgumentTypeError('speaker volume must be 0..100')
    return level


def speaker_click(level):
    """10 ms PCM click with a gentle envelope, as an in-memory WAV file."""
    rate, count = 22050, 220
    amplitude = 32767 * level / 100
    samples = [int(amplitude * min(i / 22, (count - 1 - i) / 22, 1)
                   * math.sin(2 * math.pi * 1200 * i / rate)) for i in range(count)]
    output = io.BytesIO()
    with wave.open(output, 'wb') as wav:
        wav.setnchannels(1)
        wav.setsampwidth(2)
        wav.setframerate(rate)
        wav.writeframes(struct.pack('<' + 'h' * count, *samples))
    return output.getvalue()


class AsyncSpeakerPlayer:
    """One bounded background playback; never block Tk's event loop."""
    def __init__(self):
        self.worker = None
        self.warned = False
        if sys.platform == 'win32':
            import winsound
            self.output = lambda data: winsound.PlaySound(data, winsound.SND_MEMORY | winsound.SND_NODEFAULT)
        else:
            player = shutil.which('aplay')
            self.output = (lambda data: subprocess.run([player, '-q'], input=data,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                           timeout=2, check=False)) if player else None

    def play(self, data):
        if self.output is None:
            if not self.warned:
                print('PIL DOS: PC speaker clicks unavailable (install aplay on Linux)', file=sys.stderr)
                self.warned = True
            return
        if self.worker is not None and self.worker.is_alive():
            return
        def run():
            try:
                self.output(data)
            except (OSError, RuntimeError, subprocess.SubprocessError):
                pass  # Audio failure must not stop DOS or the GUI.
        self.worker = threading.Thread(target=run, daemon=True)
        self.worker.start()


class QuietSpeaker:
    """Coalesce DOS speaker activations and throttle clicks during benchmarks."""
    def __init__(self, level, initial=0, player=None, clock=time.monotonic):
        self.level = level
        self.last_count = initial
        self.last_play = None
        self.clock = clock
        self.player = player if player is not None else (AsyncSpeakerPlayer() if level else None)
        self.click = speaker_click(level) if level else b''

    def observe(self, reply):
        count = reply.get('beeps', self.last_count)
        changed = count > self.last_count
        self.last_count = count
        if not changed or not self.level or self.player is None:
            return
        now = self.clock()
        if self.last_play is None or now - self.last_play >= SPEAKER_INTERVAL:
            self.last_play = now
            self.player.play(self.click)


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


def graphics_rgb(pixels, palette):
    """Resolve mode 13h indices through the 256-entry RGB palette."""
    indices = bytes.fromhex(pixels)
    colors = bytes.fromhex(palette)
    if len(indices) != 64000 or len(colors) != 768:
        raise ValueError('invalid mode 13h frame or palette')
    return b''.join(colors[index * 3:index * 3 + 3] for index in indices)


def cga_rgb(frame, mode, color):
    """Decode CGA even/odd scanline banks to RGB, with the 3D9 palette."""
    data = bytes.fromhex(frame)
    if len(data) != 0x4000 or mode not in (4, 5, 6) or not 0 <= color <= 255:
        raise ValueError('invalid CGA frame, mode or palette')
    colors = [bytes.fromhex(c[1:]) for c in PALETTE]
    if mode in (4, 5):
        # Mode 5 sets the CGA black-and-white bit: an RGB monitor then shows
        # cyan, red and white, and the palette-select bit has no effect.
        base = [3, 4, 7] if mode == 5 else [2, 4, 6] if not color & 0x20 else [3, 5, 7]
        palette = [colors[color & 15]] + [colors[i + (8 if color & 0x10 else 0)] for i in base]
        lut = [b''.join(palette[(b >> shift) & 3] for shift in (6, 4, 2, 0)) for b in range(256)]
    else:
        palette = [colors[0], colors[color & 15]]
        lut = [b''.join(palette[(b >> shift) & 1] for shift in range(7, -1, -1)) for b in range(256)]
    rows = ((y & 1) * 0x2000 + (y // 2) * 80 for y in range(200))
    return b''.join(lut[b] for offset in rows for b in data[offset:offset + 80])


def planar_rgb(planes, width, height, colors):
    """Combine four EGA/VGA bit planes (plane 0 first, width/8 bytes a line)
    into RGB, through the 16 colours the attribute palette and DAC give."""
    data = bytes.fromhex(planes)
    size = width // 8 * height
    palette = bytes.fromhex(colors)
    if width not in (320, 640) or len(data) != 4 * size or len(palette) != 48:
        raise ValueError('invalid planar frame or palette')
    # Each plane byte spreads into eight pixel bytes holding that plane's bit.
    spread = [[int.from_bytes(bytes((b >> (7 - i) & 1) << n for i in range(8)), 'big') for b in range(256)] for n in range(4)]
    planes4 = [data[n * size:(n + 1) * size] for n in range(4)]
    indices = b''.join((spread[0][a] | spread[1][b] | spread[2][c] | spread[3][d]).to_bytes(8, 'big')
                       for a, b, c, d in zip(*planes4))
    # Translate each channel in C rather than creating one Python object per
    # pixel. Slice assignment interleaves the channels into the PPM buffer.
    result = bytearray(len(indices) * 3)
    for channel in range(3):
        table = palette[channel::3] + bytes(240)
        result[channel::3] = indices.translate(table)
    return bytes(result)


def graphics_photo(tk, rgb, width, height=200):
    """Show 200-line CGA/VGA graphics at 4:3 using nearest-neighbour pixels.

    320x200 pixels are 5:6 and 640x200 pixels are 5:12 on a 4:3
    display. Expand each source scanline to two or three host scanlines,
    then double the width of 320-pixel modes. 350 and 480 lines are scaled
    to 480 the same way. The guest frame stays intact.
    """
    if width not in (320, 640) or height not in (200, 350, 480) or len(rgb) != width * height * 3:
        raise ValueError('invalid graphics frame dimensions')
    row_bytes = width * 3
    rows = [rgb[y * row_bytes:(y + 1) * row_bytes] for y in range(height)]
    scaled = b''.join(rows[y * height // 480] for y in range(480))
    picture = tk.PhotoImage(data=f'P6 {width} 480 255\n'.encode('ascii') + scaled, format='PPM')
    return picture.zoom(2, 1) if width == 320 else picture


def key_code(keysym, char, control=False, alt=False, altgr=False, shift=False):
    """Map a Tk key event to (ascii, scan), or None for keys DOS never sees.
    `altgr` is Ctrl and Alt down together, which is how Windows sends AltGr."""
    # Windows can hand over the Ctrl form of an AltGr character (FS for \\,
    # ESC for {, nothing for @): the key's name says which character it typed.
    if altgr and keysym in KEYSYM_CHARS and not (len(char) == 1 and char.isprintable()):
        char, control, alt = KEYSYM_CHARS[keysym], False, False
    # AltGr (Ctrl+Alt on Windows) types characters such as \\ @ { } on
    # national layouts: only Alt with a letter, digit or F-key is an Alt code.
    if alt and alt_code(keysym):
        return alt_code(keysym)
    if keysym in SPECIAL and keysym.startswith('F'):
        ascii_code, scan = SPECIAL[keysym]
        # BIOS function-key words depend on the modifier, not only 0040:0017.
        # Turbo Pascal uses F9 (4300h) to compile and Ctrl+F9 (6600h) to run.
        if control and not altgr:
            return ascii_code, scan + 0x23  # Ctrl+F1..F10: 5Eh..67h
        if shift:
            return ascii_code, scan + 0x19  # Shift+F1..F10: 54h..5Dh
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


# Tk names of the characters national layouts put on AltGr.
KEYSYM_CHARS = {'backslash': '\\', 'at': '@', 'braceleft': '{', 'braceright': '}', 'bracketleft': '[',
                'bracketright': ']', 'bar': '|', 'dollar': '$', 'asciitilde': '~', 'less': '<', 'greater': '>',
                'numbersign': '#', 'asciicircum': '^', 'grave': '`', 'sterling': '£', 'mu': 'µ'}


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


# Windows reports AltGr as Control_L and Alt_R pressed together.
ALTGR_IS_CTRL_ALT = sys.platform == 'win32'
# Modifier keys as their BIOS shift-flag bits at 0040:0017.
MODIFIERS = {'Shift_R': 0x01, 'Shift_L': 0x02, 'Control_L': 0x04, 'Control_R': 0x04, 'Alt_L': 0x08, 'Alt_R': 0x08}


def event_key_code(keysym, char, state, held):
    """Translate a key using both Tk state and tracked modifier presses."""
    # Windows sometimes clears state and retains the unmodified keysym (7),
    # while char already contains the AltGr text ({). Trust the held pair.
    if ALTGR_IS_CTRL_ALT and {'Control_L', 'Alt_R'} <= held:
        return key_code(keysym, char, altgr=True)
    control = bool(state & 0x4) or 'Control_L' in held or 'Control_R' in held
    alt = 'Alt_L' in held or ('Alt_R' in held and not control)
    altgr = control and ('Alt_L' in held or 'Alt_R' in held)
    shift = bool(state & 0x1) or 'Shift_L' in held or 'Shift_R' in held
    return key_code(keysym, char, control, alt, altgr, shift)


def is_ctrl_break(keysym, control):
    """Whether a key event is Ctrl+Break, which goes to the session as `break`."""
    return keysym in ('Cancel', 'Break') or (control and keysym == 'Pause')


def strip_vt_keys(data):
    """Host terminal input without its VT key sequences (ESC [ ... final byte,
    ESC O x); a lone ESC, as the Esc key gives, is kept."""
    out = bytearray()
    at = 0
    while at < len(data):
        byte = data[at]
        if byte == 0x1b and at + 1 < len(data) and data[at + 1] in b'[O':
            end = at + 2
            if data[at + 1] == ord('['):
                while end < len(data) and not 0x40 <= data[end] <= 0x7e:
                    end += 1
            at = end + 1
            continue
        out.append(byte)
        at += 1
    return bytes(out)


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


def is_retryable(text):
    """Whether a refused key or break waits for room rather than failing."""
    return 'buffer full' in text or 'still waiting' in text


def key_command(code):
    return 'key {} {}'.format(*code)


class KeyEvents:
    """Keep physical key releases separate while a guest owns the keyboard."""
    def __init__(self, supported):
        self.supported = supported
        self.pressed = {}

    def press(self, name, code, raw=True):
        if self.supported and raw:
            self.pressed[name] = code
            return 'keydown {} {}'.format(*code)
        return key_command(code)

    def release(self, name):
        code = self.pressed.pop(name, None)
        return 'keyup {} {}'.format(*code) if code is not None else None

    def release_all(self):
        return [self.release(name) for name in list(self.pressed)]


def defer_key_command(command, sent, settle):
    """A raw keyup needs one guest run slice, not the modifier settle delay.

    The ten-slice settle is reserved for Shift/Ctrl/Alt state transitions so
    that programs polling modifiers see a preceding key first. Applying it
    to ordinary key releases can stall a busy raw-keyboard prompt's input
    queue, especially after leaving an IDE that hooked INT 09h.
    """
    if command.startswith('shift '):
        return bool(sent or settle)
    return command.startswith('keyup ') and bool(sent)


def next_tick_delay(keys_pending, status, idle):
    """Queued host input needs a fast tick even when the DOS guest is idle."""
    return 1 if keys_pending or (status == 'running' and not idle) else 40


def mount_command(drive, path):
    letter = str(drive).strip().rstrip(':').upper()
    if letter not in ('A', 'B', 'C', 'D', 'E') or not str(path).strip() or any(c in str(path) for c in '\r\n'):
        raise ValueError(f'mount needs drive A..E and a path, got {drive!r} {path!r}')
    return f'mount {letter} {str(path).strip()}'


def unmount_command(drive):
    letter = str(drive).strip().rstrip(':').upper()
    if letter not in ('A', 'B', 'C', 'D', 'E'):
        raise ValueError(f'unmount needs drive A..E, got {drive!r}')
    return f'unmount {letter}'


def screen_rows(reply, blink_on=True):
    """Rows of (text, attributes) from a session reply's character/attribute bytes."""
    cells = bytes.fromhex(reply['screen'])
    columns = reply['columns']
    rows = []
    for r in range(len(cells) // (2 * columns)):
        row = cells[r * 2 * columns:(r + 1) * 2 * columns]
        chars, attrs = list(row[0::2]), list(row[1::2])
        for c, attr in enumerate(attrs):
            if not reply.get('video_enabled', True):
                chars[c], attrs[c] = 32, 0
            elif reply.get('text_blink', True):
                attrs[c] &= 0x7f
                if attr & 0x80 and not blink_on:
                    chars[c] = 32
        rows.append((''.join(GLYPHS[b] for b in chars), attrs))
    return rows


class Session:
    """The engine process: one command line out, one JSON reply line back."""

    def __init__(self, engine, program, mount=None, args=None, profile=None, stop_log="PILdos-stops.jsonl", ignore_unassigned_out=False, ansi=False, drives=None, config=None, fpu="none"):
        options = (['--config', str(config)] if config else []) + (['--mount', str(mount)] if mount else []) + (['--args', args] if args else []) + (['--fpu', fpu] if fpu != "none" else [])
        self.process = subprocess.Popen([str(engine), *(['--profile', str(profile)] if profile else []), *(['--ignore-unassigned-out'] if ignore_unassigned_out else []), '--stop-log', str(stop_log) if stop_log else '', '--dos-session', *options, str(program)], stdin=subprocess.PIPE,
                                        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding='utf-8',
                                        creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
        self.seq = 0
        self.com1_out = b''  # what the program sent to COM1, not yet shown
        self.last = self._read()
        if ansi:
            self.send('ansi on')
        try:
            for letter, path in (drives or []):
                reply = self.send(mount_command(letter, path))
                if not reply.get('ok'):
                    raise ValueError(reply.get('text', 'drive mount failed'))
        except Exception:
            self.close()
            raise

    def _read(self):
        line = self.process.stdout.readline(MAX_REPLY)
        if not line:
            raise RuntimeError(self.process.stderr.read().strip() or 'PIL DOS session ended')
        reply = json.loads(line)
        if reply.get('protocol') != 1 or reply.get('seq') != self.seq:
            raise RuntimeError('unexpected reply from PIL DOS session')
        self.com1_out += bytes.fromhex(reply.get('com1', ''))
        if reply.get('frame_pending'):
            reply = {**self.last, **reply}
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
            try:
                self.process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=1)
        for stream in (self.process.stdin, self.process.stdout, self.process.stderr):
            if stream:
                try:
                    stream.close()
                except OSError:
                    pass


def format_native_stats(reply):
    """Report measured compiled work; other steps also include DOS/BIOS services."""
    steps = int(reply.get('steps', 0))
    block = int(reply.get('native_block_instructions', 0))
    blocks = int(reply.get('native_blocks', 0))
    if 'native_single_instructions' not in reply:
        return ('Native single-instruction stats unavailable; rebuild the PIL engine.\\n'
                f'Guest steps: {steps:,}; compiled block instructions: {block:,}')
    single = int(reply['native_single_instructions'])
    native = block + single
    other = max(0, steps - native)
    return (
        f'Guest steps: {steps:,}\\n'
        f'Native block instructions: {block:,} in {blocks:,} blocks\\n'
        f'Native single instructions: {single:,}\\n'
        f'Native total: {native:,} ({native / steps:.1%} of steps)\\n'
        f'Other steps: {other:,} (includes DOS/BIOS services)\\n'
        f'Average native block length: {block / blocks:.2f} instructions'
        if steps and blocks else
        f'Guest steps: {steps:,}; native total: {native:,}; native blocks: {blocks:,}'
    )


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
    signed_seconds = sign * seconds
    if not (-12 * 3600 <= signed_seconds <= 14 * 3600):
        raise ValueError(f'UTC offset {text} is outside -12:00..+14:00')
    return signed_seconds


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
        self._terminal = None
        if sys.platform == 'win32':
            self._enable_vt()
            self._pass_ctrl_c()
            reader = self._read_windows
        else:
            import atexit
            import termios
            import tty
            fd = sys.stdin.fileno()
            self._terminal = (fd, termios.tcgetattr(fd))
            atexit.register(self.close)
            tty.setraw(fd)
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

    @staticmethod
    def _pass_ctrl_c():
        try:
            import atexit
            import ctypes
            kernel = ctypes.windll.kernel32
            handle = kernel.GetStdHandle(-10)  # STD_INPUT_HANDLE
            mode = ctypes.c_uint32()
            if kernel.GetConsoleMode(handle, ctypes.byref(mode)):
                old = mode.value
                kernel.SetConsoleMode(handle, old & ~0x0001)  # ENABLE_PROCESSED_INPUT off
                atexit.register(lambda: kernel.SetConsoleMode(handle, old))
        except Exception:
            pass

    def _read_windows(self):
        import msvcrt
        while True:
            char = msvcrt.getwch()
            if char in ('\x00', '\xe0'):
                msvcrt.getwch()  # DOS serial input does not understand VT key sequences
            else:
                self.input.put(to_cp437(char))

    def _read_posix(self):
        import os
        fd = sys.stdin.fileno()
        decoder = codecs.getincrementaldecoder('utf-8')('ignore')
        while True:
            data = os.read(fd, 64)
            if not data:
                tail = decoder.decode(b'', final=True)
                if tail:
                    self.input.put(to_cp437(tail))
                return
            decoded = decoder.decode(data, final=False)
            if decoded:
                self.input.put(strip_vt_keys(to_cp437(decoded)))

    def write(self, data):
        # In raw mode a bare LF only moves down; DOS sends CR LF itself.
        sys.stdout.write(from_cp437(data))
        sys.stdout.flush()

    def read(self):
        data = b''
        while not self.input.empty():
            data += self.input.get_nowait()
        return data

    def close(self):
        if self._terminal is not None:
            import termios
            fd, saved = self._terminal
            termios.tcsetattr(fd, termios.TCSADRAIN, saved)
            self._terminal = None


class TcpPort:
    """COM1 on a TCP port of this computer, for telnet or PuTTY (Raw): one
    client at a time; bytes pass both ways unchanged."""

    def __init__(self, port, host='127.0.0.1'):
        import socket
        self.server = socket.create_server((host, port))
        self.server.setblocking(False)
        self.client = None

    def _close_client(self):
        if self.client is not None:
            try:
                self.client.close()
            except OSError:
                pass
            self.client = None

    def _accept(self):
        if self.client is None and self.server is not None:
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
                self._close_client()

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
            self._close_client()
        return data.replace(b'\r\n', b'\r').replace(b'\r\x00', b'\r')

    def close(self):
        self._close_client()
        if self.server is not None:
            try:
                self.server.close()
            except OSError:
                pass
            self.server = None


def open_com1(spec):
    """`console`, or `tcp:PORT` (localhost) or `tcp:HOST:PORT`."""
    if spec == 'console':
        return ConsolePort()
    if spec.startswith('tcp:'):
        host, _, port = spec[4:].rpartition(':')
        return TcpPort(int(port), host or '127.0.0.1')
    raise ValueError(f'--com1 takes console or tcp:PORT, not {spec!r}')


def run_slice(session, budget):
    """Leave pixels in the engine until the frontend is ready to draw."""
    buffered = getattr(session, 'last', {}).get('buffered_run') is True
    return session.send(f'run {budget}' + (' buffered' if buffered else ''))


def refresh_frame(session, reply):
    return session.send('snapshot') if reply.get('frame_pending') is True else reply


class HeadlessGraphics:
    """Try to show a window only when graphics output is active."""

    def __init__(self):
        self.root = None
        self.label = None
        self.photo = None
        self.available = True
        self.user_closed = False
        self.stop_requested = False

    def update(self, reply, session=None):
        pixels = reply.get('graphics13')
        cga = reply.get('graphics_cga')
        planes = reply.get('graphics_planar')
        has_graphics = (pixels is not None or cga is not None or planes is not None)
        if not has_graphics:
            self.user_closed = False
            if self.root is not None:
                try:
                    self.root.withdraw()
                except Exception:
                    pass
            return

        if not self.available or self.user_closed:
            return

        try:
            import tkinter as tk
            if self.root is None:
                self.root = tk.Tk()
                self.root.title('PIL DOS Graphics')
                self.root.configure(bg='black')

                def on_close():
                    self.user_closed = True
                    self.stop_requested = True
                    if session is not None:
                        try:
                            session.send('break')
                            session.send(f'key 27 {SCAN.get(chr(27), 1)}')
                        except Exception:
                            pass
                    if self.root is not None:
                        try:
                            self.root.withdraw()
                        except Exception:
                            pass

                self.root.protocol('WM_DELETE_WINDOW', on_close)
                self._on_close = on_close
                self.label = tk.Label(self.root, bg='black', takefocus=1)
                self.label.pack()
                if session is not None:
                    edges = KeyEvents(session.last.get('key_edges', False))
                    def on_key(event):
                        if is_ctrl_break(event.keysym, bool(event.state & 0x4)):
                            session.send('break')
                            return
                        code = key_code(event.keysym, event.char,
                                        control=bool(event.state & 0x4),
                                        alt=bool(event.state & 0x8),
                                        shift=bool(event.state & 0x1))
                        if code:
                            session.send(edges.press(event.keysym, code, session.last.get('raw_keyboard', False)))
                    def on_release(event):
                        command = edges.release(event.keysym)
                        if command:
                            session.send(command)
                    def on_focus_out(event):
                        for command in edges.release_all():
                            session.send(command)
                    self.root.bind('<KeyPress>', on_key)
                    self.root.bind('<KeyRelease>', on_release)
                    self.root.bind('<FocusOut>', on_focus_out)

            height = 200
            if pixels is not None:
                rgb = vga_rgb(pixels, reply['palette'])
                width = 320
            elif planes is not None:
                width, height = reply['planar_width'], reply['planar_height']
                rgb = planar_rgb(planes, width, height, reply['planar_rgb'])
            else:
                mode, color = reply['cga_mode'], reply['cga_color']
                width = 320 if mode in (4, 5) else 640
                rgb = cga_rgb(cga, mode, color)

            if not reply.get('video_enabled', True):
                rgb = bytes(len(rgb))
            self.photo = graphics_photo(tk, rgb, width, height)
            self.label.configure(image=self.photo)
            self.root.deiconify()
            self.root.update_idletasks()
            self.root.update()
        except Exception:
            self.available = False

    def close(self):
        if self.root is not None:
            try:
                self.root.destroy()
            except Exception:
                pass
            self.root = None


def run_headless(session, com1, offset, country=None, is_shell=True, auto_graphics_key=False):
    """Route DOS input and output over COM1 before running the guest."""
    session.send(f'clock {offset}')
    if country:
        session.send(f'country {country}')
    reply = session.send('console com1')
    gfx = HeadlessGraphics()
    ctty_done = False
    com1_seen = False
    auto_key_sent = False
    try:
        while reply['status'] in ('running', 'waiting_input'):
            if gfx.stop_requested:
                gfx.stop_requested = False
                session.send('break')
                session.send(f'key 27 {SCAN.get(chr(27), 1)}')
                if not is_shell:
                    break
            incoming = com1.read()
            for at in range(0, len(incoming), 100):
                reply = session.send('com1 ' + incoming[at:at + 100].hex())
            reply = run_slice(session, SLICE)
            # Match the GUI's busy-run budget: avoid decoding and repainting a
            # full VGA frame after every small execution slice. Return to Tk
            # regularly so keyboard and close events stay responsive.
            until = time.monotonic() + FRAME
            while reply['status'] == 'running' and time.monotonic() < until:
                reply = run_slice(session, BURST)
            reply = refresh_frame(session, reply)
            if session.com1_out:
                com1_seen = True
                com1.write(session.com1_out)
                session.com1_out = b''
            gfx.update(reply, session)
            graphics_active = any(reply.get(k) is not None for k in ('graphics13', 'graphics_cga', 'graphics_planar'))
            if not graphics_active:
                auto_key_sent = False
            if auto_graphics_key and graphics_active and reply['status'] == 'waiting_input' and not auto_key_sent:
                result = session.send('key 13 28')
                auto_key_sent = result.get('ok', False)
                if auto_key_sent:
                    continue
            if reply['status'] == 'waiting_input' and not incoming:
                if is_shell and not ctty_done and not com1_seen:
                    ctty_done = True
                    for ch in 'ctty com1\r':
                        session.send(f'key {ord(ch)} {SCAN.get(ch.lower(), 0)}')
                    continue
                time.sleep(0.01)
        return reply
    finally:
        gfx.close()


def drive_spec(value):
    letter, sep, path = value.partition('=')
    if not sep:
        raise argparse.ArgumentTypeError('drive must be LETTER=PATH, for example A=disk.img')
    try:
        mount_command(letter, path)
    except ValueError as exc:
        raise argparse.ArgumentTypeError(str(exc)) from exc
    return (letter.strip().rstrip(':').upper(), path)


def prepare_image_drives(engine, images, drives):
    """Import images once into labelled host folders; never replace a folder."""
    drives = list(drives or [])
    used = {letter for letter, _ in drives}
    for letter, _ in images or []:
        if letter in used:
            raise ValueError(f'drive {letter}: was specified more than once')
        used.add(letter)
    for letter, image in images or []:
        def run(*args):
            result = subprocess.run([str(engine), *args], capture_output=True, text=True,
                                    encoding='utf-8', errors='replace')
            if result.returncode:
                raise ValueError(result.stderr.strip() or result.stdout.strip() or 'drive import failed')
            return result.stdout.strip()
        name = run('--dos-drive-name', image)
        if not name or any(c not in 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-' for c in name):
            raise ValueError('engine returned an invalid drive folder name')
        destination = Path('drives') / letter / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        if destination.exists():
            raise ValueError(f'{destination} already exists; reuse it with --drive {letter}={destination}')
        message = run('--dos-unpack-drive', image, str(destination))
        print(message, file=sys.stderr)
        drives.append((letter, str(destination)))
    return drives


def make_app(root, engine, program, mount=None, utc_offset=None, args=None, country=None, com1=None, keylog=False, profile=None, stop_log="PILdos-stops.jsonl", ignore_unassigned_out=False, ansi=False, speaker_level=5, drives=None, config=None, auto_graphics_key=False, fpu="none"):
    import tkinter as tk
    from tkinter import font as tkfont

    root.title(f'PIL DOS — {program}')
    face = tkfont.Font(family='Courier New' if sys.platform == 'win32' else 'Courier', size=14)
    text = tk.Text(root, width=80, height=25, font=face, bg=PALETTE[0], fg=PALETTE[7], bd=0,
                   highlightthickness=0, cursor='arrow', wrap='none')
    text.pack(padx=8, pady=(8, 4))
    graphics = tk.Label(root, bg='black', takefocus=1)
    status = tk.Label(root, anchor='w', font=('TkDefaultFont', 10))
    status.pack(fill='x', padx=8, pady=(0, 6))
    for attr in range(256):
        text.tag_configure(f'a{attr}', foreground=PALETTE[attr & 15], background=PALETTE[(attr >> 4) & 15])
    text.tag_configure('cursor', underline=True)
    # The colour tags paint every cell's background: keep the selection above
    # them, and visible when the window does not have the focus.
    text.configure(selectbackground='#3366cc', selectforeground='#ffffff', inactiveselectbackground='#3366cc')
    text.tag_raise('sel')
    session = Session(engine, program, mount, args, profile=profile, stop_log=stop_log, ignore_unassigned_out=ignore_unassigned_out, ansi=ansi, drives=drives, config=config, fpu=fpu)
    # DOS date and time come from this computer's clock in local time.
    offset = local_offset() if utc_offset is None else utc_offset
    session.send(f'clock {offset}')
    if country:
        session.send(f'country {country}')
    zone = f"UTC{'+' if offset >= 0 else '-'}{abs(offset) // 3600:02d}:{abs(offset) % 3600 // 60:02d}"
    keys = []
    edges = KeyEvents(session.last.get('key_edges', False))
    held, state = set(), {'shift': 0, 'settle': 0, 'auto_key_sent': False}
    speaker = QuietSpeaker(speaker_level, session.last.get('beeps', 0))
    geom = [80, 25]
    frame = [None]
    drawn = [None, None, None]  # [screen_hex, cursor_tuple, rows_list]
    notice = ['']  # the last refused key, shown on the status line

    def draw(reply):
        speaker.observe(reply)
        pixels = reply.get('graphics13')
        cga = reply.get('graphics_cga')
        planes = reply.get('graphics_planar')
        if pixels is not None or cga is not None or planes is not None:
            height = 200
            if pixels is not None:
                descriptor = ('vga', pixels, reply['palette'])
                width = 320
            elif planes is not None:
                descriptor = ('planar', planes, reply['planar_rgb'])
                width, height = reply['planar_width'], reply['planar_height']
            else:
                descriptor = ('cga', cga, reply['cga_mode'], reply['cga_color'])
                width = 640 if reply['cga_mode'] == 6 else 320
            descriptor += (reply.get('video_enabled', True),)
            if frame[0] != descriptor:
                if pixels is not None:
                    rgb = graphics_rgb(pixels, reply['palette'])
                elif planes is not None:
                    rgb = planar_rgb(planes, width, height, reply['planar_rgb'])
                else:
                    rgb = cga_rgb(cga, reply['cga_mode'], reply['cga_color'])
                if not reply.get('video_enabled', True):
                    rgb = bytes(len(rgb))
                picture = graphics_photo(tk, rgb, width, height)
                graphics.configure(image=picture); graphics.image = picture; frame[0] = descriptor
            if graphics.winfo_manager() != "pack":
                text.pack_forget(); graphics.pack(before=status, padx=8, pady=(8, 4)); graphics.focus_set()
            detail = f" — {reply['detail']}" if reply['detail'] else ''
            status.configure(text=("Press any key to continue" if reply['status'] == 'waiting_input' else reply['status'] + detail) + f"   steps {reply['steps']}   clock {zone}" + (f"   {notice[0]}" if notice[0] else ''))
            return
        if graphics.winfo_manager() == "pack":
            graphics.pack_forget(); text.pack(before=status, padx=8, pady=(8, 4)); text.focus_set(); drawn[0] = None
        screen_hex = (reply['screen'], reply.get('video_enabled', True), reply.get('text_blink', True), int(time.monotonic() * 2) & 1)
        cursor = tuple(reply['cursor'])
        cols = reply.get('columns', 80)
        if screen_hex != drawn[0] or cols != geom[0]:
            # Keep a selection being copied across the redraw.
            selection = text.tag_ranges('sel')
            text.configure(state='normal')
            fill(reply, drawn[2])
            if selection:
                text.tag_add('sel', *selection)
            text.configure(state='disabled')
            drawn[0] = screen_hex
            drawn[1] = cursor
        elif cursor != drawn[1]:
            text.tag_remove('cursor', '1.0', 'end')
            row, column = cursor
            text.tag_add('cursor', f'{row + 1}.{column}')
            drawn[1] = cursor
        detail = f" — {reply['detail']}" if reply['detail'] else ''
        status.configure(text=f"{reply['status']}{detail}   steps {reply['steps']}   CS:IP {reply['cs']:04x}:{reply['eip']:04x}   clock {zone}" + (f"   {notice[0]}" if notice[0] else ''))

    def insert_row(r, chars, attrs, at_end=False):
        start = 0
        for c in range(1, len(chars) + 1):
            if c == len(chars) or attrs[c] != attrs[start]:
                target = 'end' if at_end else f'{r + 1}.{start}'
                text.insert(target, chars[start:c], f'a{attrs[start]}')
                start = c

    def fill(reply, prev_rows):
        rows = screen_rows(reply, blink_on=bool(int(time.monotonic() * 2) & 1))
        cols = reply.get('columns', 80)
        nrows = len(rows) or 25
        if [cols, nrows] != geom or prev_rows is None or len(prev_rows) != len(rows):
            geom[:] = [cols, nrows]
            text.configure(width=cols, height=nrows)
            text.delete('1.0', 'end')
            for r, (chars, attrs) in enumerate(rows):
                insert_row(r, chars, attrs, at_end=True)
                if r < len(rows) - 1:
                    text.insert('end', '\n')
        else:
            text.tag_remove('cursor', '1.0', 'end')
            for r, row_data in enumerate(rows):
                if row_data != prev_rows[r]:
                    text.delete(f'{r + 1}.0', f'{r + 1}.end')
                    insert_row(r, row_data[0], row_data[1], at_end=False)
        drawn[2] = rows
        row, column = reply['cursor']
        if reply.get('video_enabled', True):
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
        if keylog:
            print(f'key {event.keysym!r} char {event.char!r} state {event.state:#x} keycode {event.keycode}'
                  f' held {sorted(held)}', file=sys.stderr, flush=True)
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
        if is_ctrl_break(event.keysym, control):
            keys.append('break')
            return 'break'
        code = event_key_code(event.keysym, event.char, event.state, held)
        if code:
            keys.append(edges.press(event.keysym, code, session.last.get('raw_keyboard', False)))
        return 'break'

    def on_release(event):
        if keylog:
            print(f'release {event.keysym!r} state {event.state:#x}', file=sys.stderr, flush=True)
        command = edges.release(event.keysym)
        if command:
            keys.append(command)
        held.discard(event.keysym)
        return shift_changed()

    def shift_changed():
        flags = 0
        for name in held:
            flags |= MODIFIERS[name]
        # Windows sends AltGr as Control_L with Alt_R. DOS never saw that Ctrl,
        # and a program reading the shift flags would take the character typed
        # with AltGr for a Ctrl+Alt key: report neither while AltGr is down.
        if ALTGR_IS_CTRL_ALT and {'Control_L', 'Alt_R'} <= held:
            flags &= ~0x0c
            if 'Control_R' in held:
                flags |= 0x04
        if flags != state['shift']:
            state['shift'] = flags
            keys.append(f'shift {flags}')
        return 'break'

    def lost_focus(event):
        keys.extend(edges.release_all())
        held.clear()
        shift_changed()

    def tick():
        try:
            reply = session.last
            previous = reply
            sent = False
            state['settle'] = max(0, state['settle'] - 1)
            while keys and reply['status'] in ('running', 'waiting_input'):
                # A change of the shift keys waits until the program has run a
                # while after the keys before it, so that EDIT reads Alt+F
                # before it sees Alt go up, as it would at a person's pace.
                if defer_key_command(keys[0], sent, state['settle']):
                    break
                reply = session.send(keys[0])
                if not reply['ok']:
                    # A full keyboard is retried on a later tick; any other
                    # refusal (such as Ctrl+Break the session cannot deliver)
                    # is shown and dropped, so keys behind it still arrive.
                    if not is_retryable(reply.get('text', '')):
                        notice[0] = f"{keys.pop(0)}: {reply.get('text', '')}"
                    break
                sent = keys.pop(0)
                if sent.startswith('shift'):
                    break
                state['settle'] = SETTLE
            incoming = b''
            if com1:
                incoming = com1.read()
                for at in range(0, len(incoming), 100):
                    if reply['status'] in ('running', 'waiting_input'):
                        reply = session.send('com1 ' + incoming[at:at + 100].hex())
            if reply['status'] in ('running', 'waiting_input'):
                reply = run_slice(session, SLICE)
                until = time.monotonic() + FRAME
                idle = (not sent and not keys and not incoming and reply.get('idle_hint', False) and
                        all(reply.get(k) == previous.get(k) for k in
                            ('screen', 'graphics_planar', 'graphics_cga', 'graphics13')))
                while reply['status'] == 'running' and not keys and not idle and time.monotonic() < until:
                    speaker.observe(reply)
                    reply = run_slice(session, BURST)
            else:
                idle = False
            reply = refresh_frame(session, reply)
            if com1 and session.com1_out:
                com1.write(session.com1_out)
                session.com1_out = b''
            if reply['status'] == 'exit':
                close()
                return
            draw(reply)
            graphics_active = any(reply.get(k) is not None for k in ('graphics13', 'graphics_cga', 'graphics_planar'))
            if not graphics_active:
                state['auto_key_sent'] = False
            if auto_graphics_key and graphics_active and reply['status'] == 'waiting_input' and not state['auto_key_sent']:
                result = session.send('key 13 28')
                state['auto_key_sent'] = result.get('ok', False)
        except (OSError, RuntimeError, ValueError) as error:
            status.configure(text=f'session error: {error}')
            return
        root.after(next_tick_delay(bool(keys), session.last['status'], idle), tick)

    def close():
        if com1 and hasattr(com1, 'close'):
            com1.close()
        session.close()
        root.destroy()

    def mount_image(letter):
        from tkinter import filedialog
        chosen = filedialog.askopenfilename(
            parent=root, title=f'Mount floppy or disk image on {letter}:',
            filetypes=[('Disk images', '*.img *.ima *.IMG *.IMA'), ('All files', '*.*')])
        if chosen:
            reply = session.send(mount_command(letter, chosen))
            root.title(f"PIL DOS — {program} ({reply.get('text', '')})")
            draw(reply)

    def mount_folder(letter):
        from tkinter import filedialog
        chosen = filedialog.askdirectory(parent=root, title=f'Mount host folder on {letter}:')
        if chosen:
            reply = session.send(mount_command(letter, chosen))
            root.title(f"PIL DOS — {program} ({reply.get('text', '')})")
            draw(reply)

    def unmount_drive(letter):
        reply = session.send(unmount_command(letter))
        root.title(f"PIL DOS — {program} ({reply.get('text', '')})")
        draw(reply)

    root.bind('<Key>', on_key)
    root.bind('<KeyRelease>', on_release)
    root.bind('<FocusOut>', lost_focus)
    menu = tk.Menu(root, tearoff=0)
    menu.add_command(label='Copy (Ctrl+Shift+C)', command=copy)
    menu.add_command(label='Paste (Ctrl+Shift+V)', command=paste)
    menu.add_separator()
    drives_menu = tk.Menu(menu, tearoff=0)
    for drv in ('A', 'B', 'D', 'E'):
        sub = tk.Menu(drives_menu, tearoff=0)
        sub.add_command(label=f'Mount Floppy / Image (.img/.ima) on {drv}:...', command=lambda d=drv: mount_image(d))
        sub.add_command(label=f'Mount Host Folder on {drv}:...', command=lambda d=drv: mount_folder(d))
        sub.add_command(label=f'Unmount {drv}:', command=lambda d=drv: unmount_drive(d))
        drives_menu.add_cascade(label=f'Drive {drv}:', menu=sub)
    menu.add_cascade(label='Drives', menu=drives_menu)
    for widget in (text, graphics, status):
        widget.bind('<Button-3>', lambda event: menu.tk_popup(event.x_root, event.y_root))
    root.protocol('WM_DELETE_WINDOW', close)
    draw(session.last)
    root.after(10, tick)
    return session


def configured_startup(engine, config=None):
    command = [str(engine), '--pildos-startup'] + ([str(config)] if config else [])
    result = subprocess.run(command, capture_output=True, text=True, encoding='utf-8', errors='replace')
    if result.returncode:
        raise ValueError(result.stderr.strip() or 'PILdos startup configuration failed')
    try:
        value = json.loads(result.stdout)
        program, args = value['program'], value['args']
        if not isinstance(program, str) or not program or (args is not None and not isinstance(args, str)):
            raise ValueError('invalid startup values')
        return program, args
    except (ValueError, KeyError, TypeError) as exc:
        raise ValueError('invalid PILdos startup configuration response') from exc


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('program', nargs='?', help='DOS .COM/.EXE, or IMAGE.img::INNER_PATH; default from config')
    parser.add_argument('--mount', help='host folder shown as the next drive (D: after an image); DOS writes reach it')
    parser.add_argument('--config', help='DOS configuration file (default: pildos.conf if present)')
    parser.add_argument('--drive', action='append', type=drive_spec, metavar='LETTER=PATH',
                        help='mount an image or folder on drive A through E before startup; repeat for more drives')
    parser.add_argument('--drive-from-image', action='append', type=drive_spec, metavar='LETTER=IMAGE',
                        help='copy a FAT image to a new labelled drives/LETTER folder and mount it writable')
    parser.add_argument('--args', metavar='COMMAND_LINE',
                        help='command line for the program, e.g. "/K D:\\START.BAT" to have COMMAND.COM run a batch file first')
    parser.add_argument('--speaker-volume', type=speaker_volume, default=5, metavar='0..100',
                        help='PC speaker click volume (default 5, 0 mutes; Windows or Linux with aplay)')
    parser.add_argument('--fpu', choices=('none', '80387'), default='none',
                        help='optional partial 80387 backend (default none; arithmetic is not yet implemented)')
    parser.add_argument('--ansi', action='store_true',
                        help='load an ANSI.SYS-style console driver, so ESC [ colour and cursor sequences draw on the DOS screen')
    parser.add_argument('--country', type=int, choices=[1, 358],
                        help='DOS country: 1 (USA, the default: 10-31-2026, 2:27p) or 358 (Finland: 31.10.2026, 14.27)')
    parser.add_argument('--com1', metavar='console|tcp:PORT',
                        help='connect COM1 to the console this was started from, or to a TCP port for telnet/PuTTY')
    parser.add_argument('--headless', action='store_true',
                        help='run without a window, interacting through this terminal (shows a window only when graphics output is active)')
    parser.add_argument('--auto-graphics-key', action='store_true',
                        help='send Enter at a graphics program key wait (useful with DOS TIMEIT)')
    parser.add_argument('--engine', help='pil executable (default: newest build)')
    parser.add_argument('--ignore-unassigned-out', action='store_true', help='ignore and log OUT to unassigned ports; known devices still stop')
    parser.add_argument('--stop-log', default='PILdos-stops.jsonl', metavar='PATH', help='append stop diagnostics (default: PILdos-stops.jsonl)')
    parser.add_argument('--profile', metavar='JSON_PATH', help='write engine timings when the session closes')
    parser.add_argument('--native-stats', action='store_true', help='print native block/single execution coverage when the DOS window closes')
    parser.add_argument('--keylog', action='store_true',
                        help='print each key event the window gets (name, character, state) to the console')
    parser.add_argument('--utc-offset', type=parse_offset, metavar='HOURS',
                        help='time zone of the DOS clock, e.g. 2, -5, 5.5 or +02:00 (default: this computer\'s)')
    args = parser.parse_args(argv)
    if args.program is None:
        try:
            args.program, default_args = configured_startup(find_dos_engine(args.engine), args.config)
            if args.args is None:
                args.args = default_args
        except (OSError, ValueError) as exc:
            parser.error(str(exc))
    if args.drive_from_image:
        try:
            args.drive = prepare_image_drives(find_dos_engine(args.engine), args.drive_from_image, args.drive)
        except (OSError, ValueError) as exc:
            parser.error(str(exc))
    if args.headless:
        if args.com1 is None:
            args.com1 = 'console'
        if args.args is None:
            args.args = '/K CTTY COM1'
    com1 = open_com1(args.com1) if args.com1 else None
    if args.headless:
        session = None
        try:
            session = Session(find_dos_engine(args.engine), args.program, args.mount, args.args, profile=args.profile,
                              stop_log=args.stop_log, ignore_unassigned_out=args.ignore_unassigned_out, ansi=args.ansi, drives=args.drive, config=args.config, fpu=args.fpu)
            is_shell = any(sh in args.program.upper() for sh in ('COMMAND.COM', 'CMD.EXE', '4DOS'))
            reply = run_headless(session, com1, local_offset() if args.utc_offset is None else args.utc_offset, args.country, is_shell=is_shell, auto_graphics_key=args.auto_graphics_key)
            if reply['status'] != 'exit':
                print(f"\nPIL stopped: {reply.get('detail', reply['status'])}", file=sys.stderr)
                raise SystemExit(1)
        finally:
            if session:
                if args.native_stats:
                    print(format_native_stats(session.last))
                session.close()
            com1.close()
        return
    import tkinter as tk
    root = tk.Tk()
    session = make_app(root, find_dos_engine(args.engine), args.program, args.mount, args.utc_offset, args.args, args.country, com1, args.keylog, args.profile, args.stop_log, args.ignore_unassigned_out, args.ansi, args.speaker_volume, args.drive, args.config, args.auto_graphics_key, fpu=args.fpu)
    root.mainloop()
    if args.native_stats:
        print(format_native_stats(session.last))


if __name__ == '__main__':
    main()

