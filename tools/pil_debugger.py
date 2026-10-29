#!/usr/bin/env python3
"""Optional Tk frontend for PIL's Rust debugger service (Python 3.10+).

Original implementation, drawing layout/event-routing ideas from the user's
lispy3 and MIT z80-cpm projects. No interpreter or guest services are duplicated.
"""
import argparse
import json
from pathlib import Path
import queue
import subprocess
import sys
import tempfile
import threading

MAX_REPLY = 262144
MAX_MONITOR_BYTES = 3840
MAX_HEX_TEXT_BYTES = 65536
STATUSES = {'ready', 'stepped', 'budget', 'breakpoint', 'waiting_rx', 'tx_blocked',
            'halt', 'fault', 'decode', 'missing_instruction', 'error'}


class ProtocolError(RuntimeError):
    pass


class AsyncJobRunner:
    """Runs bounded helper subprocesses off the Tk main thread and drains callbacks on poll()."""
    def __init__(self):
        self.completed = queue.Queue()
        self._lock = threading.Lock()
        self.active = False

    def submit(self, fn, callback):
        with self._lock:
            if self.active:
                return False
            self.active = True

        def worker():
            try:
                result = fn()
                error = None
            except Exception as exc:
                result = None
                error = exc
            with self._lock:
                self.active = False
            self.completed.put((callback, result, error))

        threading.Thread(target=worker, daemon=True).start()
        return True

    def drain(self):
        count = 0
        while True:
            try:
                callback, result, error = self.completed.get_nowait()
            except queue.Empty:
                break
            callback(result, error)
            count += 1
        return count


def complete_monitor_load(result, error, *, is_current, load, report_error):
    """Deliver on the UI thread only while the requesting session/browser is live."""
    if not is_current():
        if result is not None:
            Path(result[0]).unlink(missing_ok=True)
        return
    if error is not None:
        report_error(error)
        return
    load(*result)


def parse_container_listing(stdout: str):
    """Parse `--dos-list` output lines into (entry_path, size_bytes, kind)."""
    entries = []
    for line in stdout.splitlines():
        line = line.strip()
        if not line or line.startswith('NOTE ') or line.startswith('CONTAINER '):
            continue
        parts = line.split(None, 1)
        if len(parts) == 2 and parts[0].isdigit():
            size = int(parts[0])
            ent_path = parts[1]
            ext = Path(ent_path).suffix.lower()
            kind = 'COM' if ext == '.com' else ('EXE' if ext == '.exe' else ('ZIP' if ext == '.zip' else 'FILE'))
            entries.append((ent_path, size, kind))
    return entries


def _verify_cputest_com_matches_manifest(com_path: Path, validated: list) -> bool:
    """Verify that `CPUTEST.COM` was built from `validated` (`cases.json`)."""
    try:
        size = com_path.stat().st_size
        if not (1 <= size <= 0xff00):
            return False
        com_bytes = com_path.read_bytes()
    except OSError:
        return False
    if b'PIL-DOSBOX CPU CHECK v1\r\n$' not in com_bytes:
        return False
    if f'COMPLETE {len(validated)} CASES\r\n$'.encode('ascii') not in com_bytes:
        return False
    u32 = lambda v: type(v) is int and 0 <= v <= 0xffffffff
    for case in validated:
        test_eip = case.get('test_eip')
        done_eip = case.get('done_eip')
        if not (u32(test_eip) and u32(done_eip)):
            return False
        code_bytes = bytes.fromhex(case['code'])
        start_off = test_eip - 0x100
        end_off = start_off + len(code_bytes)
        if not (0 <= start_off < end_off <= done_eip - 0x100 <= len(com_bytes)):
            return False
        if com_bytes[start_off:end_off] != code_bytes:
            return False
    return True


def load_case_manifest(path):
    """Load and validate `cases.json` explicitly, or from an authenticated `CPUTEST.COM`."""
    if not path:
        return []
    p = Path(path)
    is_json = p.suffix.lower() == '.json'
    if is_json:
        candidate = p
    elif p.name.upper() == 'CPUTEST.COM':
        candidate = p.with_name('cases.json')
    else:
        return []
    if not candidate.is_file():
        return []
    try:
        if candidate.stat().st_size > 4 * 1024 * 1024:
            return []
        raw = json.loads(candidate.read_text(encoding='utf-8'))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError):
        return []
    if not isinstance(raw, list) or not (1 <= len(raw) <= 4096):
        return []
    u32 = lambda v: type(v) is int and 0 <= v <= 0xffffffff
    validated = []
    for idx, item in enumerate(raw, 1):
        if not isinstance(item, dict) or not isinstance(item.get('name'), str):
            return []
        if not all(u32(item.get(k)) for k in ('eax', 'ecx', 'edx', 'expected_eax', 'expected_edx', 'flags', 'flags_mask', 'incoming_flags')):
            return []
        code_hex = item.get('code')
        if not isinstance(code_hex, str):
            return []
        try:
            code_bytes = bytes.fromhex(code_hex)
        except ValueError:
            return []
        if not (1 <= len(code_bytes) <= 15):
            return []
        entry = dict(item)
        entry['index'] = item.get('index') if u32(item.get('index')) and item.get('index') > 0 else idx
        entry['expected_ecx'] = item.get('expected_ecx') if u32(item.get('expected_ecx')) else item['ecx']
        validated.append(entry)
    if not is_json and not _verify_cputest_com_matches_manifest(p, validated):
        return []
    return validated


def build_monitor_case(case):
    """Build a self-contained monitor binary (<=MAX_MONITOR_BYTES) for a single test case."""
    code = bytearray()

    def mov32(reg, val):
        code.extend(bytes([0x66, 0xb8 + reg]) + int(val & 0xffffffff).to_bytes(4, 'little'))

    def cmp32(reg, val):
        code.extend(bytes([0x66, 0x81, 0xf8 + reg]) + int(val & 0xffffffff).to_bytes(4, 'little'))

    # SS=0x20 in the 4KB monitor -> linear base 0x200; SP=0x0dfe -> linear 0x0ffe.
    mov32(4, 0x00000dfe)
    for reg, val in [
        (0, case['eax']),
        (1, case['ecx']),
        (2, case['edx']),
        (3, 0xdead0000),
        (6, 0x11223344),
        (7, 0x55667788),
    ]:
        mov32(reg, val)
    code.append(0x68)
    code.extend(int(case['incoming_flags'] & 0xffff).to_bytes(2, 'little'))
    code.append(0x9d)  # POPF
    test_eip = 0x100 + len(code)
    code.extend(bytes.fromhex(case['code']))
    check_eip = 0x100 + len(code)
    code.extend(b'\x9c\x5b\xfc')  # PUSHF; POP BX; CLD
    fixups = []
    for reg, exp in [
        (0, case['expected_eax']),
        (1, case.get('expected_ecx', case['ecx'])),
        (2, case['expected_edx']),
        (6, 0x11223344),
        (7, 0x55667788),
    ]:
        cmp32(reg, exp)
        code.extend(b'\x0f\x85')
        fixups.append(len(code))
        code.extend(b'\x00\x00')
    code.extend(b'\x81\xe3')
    code.extend(int(case['flags_mask'] & 0xffff).to_bytes(2, 'little'))
    code.extend(b'\x81\xfb')
    code.extend(int((case['flags'] & case['flags_mask']) & 0xffff).to_bytes(2, 'little'))
    code.extend(b'\x0f\x85')
    fixups.append(len(code))
    code.extend(b'\x00\x00')
    pass_eip = 0x100 + len(code)
    mov32(0, 0)
    code.append(0xf4)  # HLT on PASS (EAX=0)
    fail_pos = len(code)
    fail_eip = 0x100 + fail_pos
    mov32(0, int(case.get('index', 1)))
    code.append(0xf4)  # HLT on FAIL (EAX=index, BX=masked flags)
    for pos in fixups:
        rel = (fail_pos - (pos + 2)) & 0xffff
        code[pos:pos + 2] = rel.to_bytes(2, 'little')
    meta = {
        'index': int(case.get('index', 1)),
        'name': case['name'],
        'test_eip': test_eip,
        'check_eip': check_eip,
        'pass_eip': pass_eip,
        'fail_eip': fail_eip,
        'expected_eax': case['expected_eax'],
        'expected_ecx': case.get('expected_ecx', case['ecx']),
        'expected_edx': case['expected_edx'],
        'expected_flags': case['flags'] & case['flags_mask'],
        'flags_mask': case['flags_mask'],
    }
    return bytes(code), meta


def format_case_inspector(meta, eip):
    """Format a compact bisection summary for the active monitor test case."""
    if not isinstance(meta, dict):
        return ''
    if eip == meta['test_eip']:
        phase = 'AT TEST INSTRUCTION'
    elif eip in (meta['pass_eip'], meta['pass_eip'] + 6):
        phase = 'PASSED'
    elif eip in (meta['fail_eip'], meta['fail_eip'] + 6):
        phase = 'FAILED'
    elif eip >= meta['check_eip']:
        phase = 'checking'
    else:
        phase = 'setup'
    return (
        f"Case #{meta['index']:03d} {meta['name']} [{phase}] "
        f"(test=0x{meta['test_eip']:04x} pass=0x{meta['pass_eip']:04x} fail=0x{meta['fail_eip']:04x}) "
        f"exp EAX={meta['expected_eax']:08X} ECX={meta['expected_ecx']:08X} "
        f"EDX={meta['expected_edx']:08X} FLAGS&{meta['flags_mask']:04X}={meta['expected_flags']:04X}"
    )


class CommandHistory:
    """Bounded command history for the listener, navigating previous/next entries."""
    def __init__(self, limit=100):
        self.limit = limit
        self.entries = []
        self.index = 0

    def append(self, command):
        command = command.strip()
        if not command:
            return
        if not self.entries or self.entries[-1] != command:
            self.entries.append(command)
            if len(self.entries) > self.limit:
                del self.entries[0]
        self.index = len(self.entries)

    def previous(self):
        if not self.entries:
            return None
        if self.index > 0:
            self.index -= 1
        return self.entries[self.index]

    def next(self):
        if not self.entries:
            return None
        if self.index < len(self.entries) - 1:
            self.index += 1
            return self.entries[self.index]
        self.index = len(self.entries)
        return ''


def parse_hex_program(text: str, max_bytes: int = MAX_MONITOR_BYTES) -> bytes:
    """Parse whitespace-separated hex bytes, ignoring comments starting with ';' or '#'."""
    raw = bytearray()
    for line in text.splitlines():
        line = line.split(';', 1)[0].split('#', 1)[0].strip()
        for token in line.split():
            token = token.removeprefix('0x').removeprefix('0X')
            try:
                val = int(token, 16)
                if not 0 <= val <= 255:
                    raise ValueError(f'Byte value out of range: {token}')
                raw.append(val)
            except ValueError:
                raise ValueError(f'Invalid hex byte: {token!r}') from None
            if len(raw) > max_bytes:
                raise ValueError(f'Program exceeds {max_bytes} byte monitor limit')
    if not raw:
        raise ValueError('No hex bytes found in file')
    return bytes(raw)


def read_bounded_hex_file(path, max_bytes: int = MAX_MONITOR_BYTES, max_text_bytes: int = MAX_HEX_TEXT_BYTES) -> bytes:
    """Read at most max_text_bytes + 1 bytes from a .hex file before parsing."""
    with open(path, 'rb') as handle:
        chunk = handle.read(max_text_bytes + 1)
    if len(chunk) > max_text_bytes:
        raise ValueError(f'Hex file exceeds {max_text_bytes} byte text limit')
    return parse_hex_program(chunk.decode('utf-8', errors='replace'), max_bytes=max_bytes)


def format_memory_cell_value(effect) -> str:
    """Format the affected memory cell value in its appropriate byte width and character count."""
    if not isinstance(effect, dict):
        return 'Affected cell: none (current instruction uses registers only)'
    raw = effect.get('raw') or []
    addr = effect.get('address', 0)
    n = effect.get('bytes', len(raw))
    if not raw or n <= 0:
        return 'Affected cell: none'
    uval = int.from_bytes(bytes(raw), 'little', signed=False)
    sval = int.from_bytes(bytes(raw), 'little', signed=True)
    hex_val = f'0x{uval:0{n * 2}X}'
    hex_bytes = ' '.join(f'{b:02x}' for b in raw)
    ascii_bar = ''.join(chr(b) if 32 <= b <= 126 else '.' for b in raw)
    char_lits = ' '.join(f"'{chr(b)}'" if 32 <= b <= 126 else f"'\\x{b:02x}'" for b in raw)
    span = f'0x{addr:08X}' if n <= 1 else f'0x{addr:08X}..0x{addr + n - 1:08X}'
    unit = '1 byte' if n == 1 else f'{n} bytes'
    stage = effect.get('stage', '')
    stage_label = {
        'written': 'written',
        'read': 'read',
        'next_write': 'target write',
        'next_read': 'target read',
    }.get(stage, stage)
    dec_str = f'{uval:,} (signed {sval:,})' if sval < 0 else f'{uval:,}'
    return (
        f'Cell [{span}] ({unit}, {stage_label}) · Hex: {hex_val} · Dec: {dec_str} '
        f'· Bytes: [{hex_bytes}] · Chars ({len(raw)}): |{ascii_bar}| ({char_lits})'
    )


def find_memory_highlight_spans(dump_text: str, effect):
    """Return Tk text index spans [(start, end), ...] for both hex bytes and ASCII chars of the affected cell."""
    if not isinstance(effect, dict) or not dump_text:
        return []
    cell_start = effect.get('address')
    n = effect.get('bytes', 0)
    if type(cell_start) is not int or type(n) is not int or n <= 0:
        return []
    cell_end = cell_start + n
    spans = []
    for row, line in enumerate(dump_text.splitlines(), 1):
        if ':' not in line:
            continue
        try:
            row_addr = int(line.split(':', 1)[0].strip(), 16)
        except ValueError:
            continue
        lo = max(row_addr, cell_start)
        hi = min(row_addr + 16, cell_end)
        if lo >= hi:
            continue
        start_col = lo - row_addr
        end_col = hi - row_addr
        hex_start = 10 + start_col * 3
        hex_end = 10 + (end_col - 1) * 3 + 2
        spans.append((f'{row}.{hex_start}', f'{row}.{hex_end}'))
        pipe_idx = line.find('|')
        if pipe_idx >= 0:
            ascii_start = pipe_idx + 1 + start_col
            ascii_end = pipe_idx + 1 + end_col
            spans.append((f'{row}.{ascii_start}', f'{row}.{ascii_end}'))
    return spans


def find_changed_memory_spans(prev_dump: str, curr_dump: str):
    """Return Tk text index spans for individual bytes that changed at the same physical address."""
    if not prev_dump or not curr_dump:
        return []
    prev_rows = {}
    for line in prev_dump.splitlines():
        if ':' not in line:
            continue
        try:
            row_addr = int(line.split(':', 1)[0].strip(), 16)
        except ValueError:
            continue
        hex_part = line[10:57].split()
        if len(hex_part) == 16:
            prev_rows[row_addr] = hex_part
    spans = []
    for row, line in enumerate(curr_dump.splitlines(), 1):
        if ':' not in line:
            continue
        try:
            row_addr = int(line.split(':', 1)[0].strip(), 16)
        except ValueError:
            continue
        if row_addr not in prev_rows:
            continue
        old_bytes = prev_rows[row_addr]
        new_bytes = line[10:57].split()
        if len(new_bytes) != 16:
            continue
        pipe_idx = line.find('|')
        for col in range(16):
            if old_bytes[col] != new_bytes[col]:
                spans.append((f'{row}.{10 + col * 3}', f'{row}.{10 + col * 3 + 2}'))
                if pipe_idx >= 0:
                    spans.append((f'{row}.{pipe_idx + 1 + col}', f'{row}.{pipe_idx + 2 + col}'))
    return spans


def decode_message(line, expected_seq):
    if len(line.encode('utf-8')) > MAX_REPLY or not line.endswith('\n'):
        raise ProtocolError('Oversized or incomplete debugger reply')
    message = json.loads(line)
    if (not isinstance(message, dict) or message.get('protocol') != 1
            or type(message.get('seq')) is not int or message['seq'] != expected_seq
            or type(message.get('ok')) is not bool or type(message.get('quit')) is not bool
            or not isinstance(message.get('text'), str) or message.get('status') not in STATUSES):
        raise ProtocolError('Unsupported or out-of-order debugger reply')
    state, data = message.get('state'), message.get('guest_output')
    integer = lambda n: type(n) is int and 0 <= n <= 0xffffffff
    if (not isinstance(state, dict) or not isinstance(state.get('registers'), list)
            or len(state['registers']) != 8 or not all(map(integer, state['registers']))
            or not all(integer(state.get(key)) for key in ('eip', 'flags', 'undefined_flags'))
            or not isinstance(data, list) or len(data) > 256
            or not all(type(b) is int and 0 <= b <= 255 for b in data)):
        raise ProtocolError('Invalid debugger state or guest bytes')
    if (not all(isinstance(state.get(key), str) for key in
                ('version', 'demo', 'listing', 'memory', 'stack', 'segments', 'cache'))
            or not all(integer(state.get(key)) for key in ('memory_address', 'stack_address', 'ram_size'))
            or not isinstance(state.get('breakpoints'), list)
            or not all(map(integer, state['breakpoints']))
            or ('pil' in state and not isinstance(state['pil'], str))):
        raise ProtocolError('Incomplete debugger snapshot')
    return message


class EngineClient:
    """One owned backend, one outstanding request, bounded UI event queue."""
    def __init__(self, argv, raw_path=None):
        self.events = queue.Queue(maxsize=8)
        self.commands = queue.Queue(maxsize=1)
        self.closed = threading.Event()
        self._lock = threading.Lock()
        self._pending = True
        self.stderr = tempfile.TemporaryFile()
        try:
            self.process = subprocess.Popen(
                [*map(str, argv), '--debug-service', *(['--load', str(raw_path)] if raw_path else [])],
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.stderr,
                text=True, encoding='utf-8', errors='strict', bufsize=1,
                creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
        except Exception:
            self.stderr.close()
            raise
        self.thread = threading.Thread(target=self._work, daemon=True)
        self.thread.start()

    @property
    def pending(self):
        with self._lock:
            return self._pending

    def request(self, command):
        if '\n' in command or '\r' in command or len(command.encode('utf-8')) > 4095:
            raise ValueError('Enter one command, at most4095 UTF-8 bytes')
        with self._lock:
            if self.closed.is_set():
                raise RuntimeError('Debugger session is closed')
            if self._pending:
                return False
            self._pending = True
        self.commands.put_nowait(command)
        return True

    def _post(self, item):
        while not self.closed.is_set():
            try:
                self.events.put(item, timeout=.1)
                return
            except queue.Full:
                pass

    def _work(self):
        seq, command = 0, ''
        try:
            while not self.closed.is_set():
                line = self.process.stdout.readline(MAX_REPLY + 1)
                if not line:
                    self.process.wait(timeout=2)
                    self.stderr.seek(0)
                    detail = self.stderr.read(8192).decode('utf-8', errors='replace').strip()
                    raise ProtocolError(detail or 'Debugger process disconnected')
                message = decode_message(line, seq)
                with self._lock:
                    self._pending = False
                self._post(('reply', command, message))
                if message['quit']:
                    return
                while not self.closed.is_set():
                    try:
                        command = self.commands.get(timeout=.1)
                        break
                    except queue.Empty:
                        continue
                else:
                    return
                self.process.stdin.write(command + '\n')
                self.process.stdin.flush()
                seq += 1
        except Exception as error:
            self._post(('error', command, str(error)))
        finally:
            self.closed.set()
            if self.process.poll() is None:
                self.process.terminate()
            try:
                self.process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait()
            for stream in (self.process.stdin, self.process.stdout, self.stderr):
                stream.close()

    def close(self):
        self.closed.set()
        if self.process.poll() is None:
            self.process.terminate()
        self.thread.join(timeout=.3)


def next_command(running, message):
    if not running or not message['ok']:
        return None
    if message['status'] == 'tx_blocked':
        return 'serial tx'
    if message['status'] in {'budget', 'stepped', 'ready'}:
        return 'run 256'
    return None


def find_engine(explicit=None):
    if explicit:
        path = Path(explicit).expanduser().resolve()
        if not path.is_file():
            raise FileNotFoundError(f'PIL executable not found: {path}')
        return path
    name = 'pil-proof.exe' if sys.platform == 'win32' else 'pil-proof'
    found = []
    for root in [Path(__file__).resolve().parent, Path(__file__).resolve().parent.parent]:
        for path in [root / name, root / 'target/release' / name, root / 'target/debug' / name]:
            if path.is_file():
                found.append(path.resolve())
    if found:
        return max(found, key=lambda p: p.stat().st_mtime)
    raise FileNotFoundError('Build PIL first, or select its executable with --engine PATH')


def make_app(root, engine, raw_path=None):
    import tkinter as tk
    from tkinter import ttk, filedialog, messagebox

    class App:
        def __init__(self):
            self.root, self.engine = root, engine
            self.client = None
            self.busy, self.running, self.closing = True, False, False
            self.state, self.previous, self.guest = {}, {}, bytearray()
            self.history = CommandHistory()
            self.jobs = AsyncJobRunner()
            self.cases = []
            self.active_case_meta = None
            self._pending_case_index = None
            self._temp_file = None
            self._last_dir = str(Path(__file__).resolve().parents[1] / 'programs')
            root.title('PIL debugger')
            root.geometry('1180x780')
            root.minsize(850, 560)
            style = ttk.Style(root)
            if 'clam' in style.theme_names():
                style.theme_use('clam')
            bar = ttk.Frame(root, padding=6)
            bar.pack(fill='x')
            for label, callback in [('Open program…', self.open_file),
                                    ('Open from image…', self.open_image_dialog),
                                    ('Step F7', lambda: self.send('step')),
                                    ('Step PIL F8', lambda: self.send('pilstep')),
                                    ('Run F5', self.run), ('Pause F6', self.pause),
                                    ('Reset', lambda: self.send('reset')),
                                    ('Check', lambda: self.send('check'))]:
                ttk.Button(bar, text=label, command=callback).pack(side='left', padx=3)
            self.demo = tk.StringVar(value='counter')
            chooser = ttk.Combobox(bar, textvariable=self.demo, state='readonly', width=14,
                                  values=['counter', 'echo', 'memory', 'selftest', 'hello',
                                          'factorials', 'loop', 'mandelbrot', 'extensions'])
            chooser.pack(side='right')
            chooser.bind('<<ComboboxSelected>>', self.on_demo_selected)
            pane = ttk.Panedwindow(root, orient='horizontal')
            pane.pack(fill='both', expand=True, padx=6)
            left, right = ttk.Frame(pane), ttk.Frame(pane)
            pane.add(left, weight=3)
            pane.add(right, weight=2)
            self.code = self.text(left, 'Code · current instruction marked * · double-click breakpoint', height=14)
            self.code.tag_configure('current', background='#d9eafa', foreground='#102030')
            self.code.tag_configure('breakpoint', foreground='#b02020')
            self.code.bind('<Double-Button-1>', self.code_breakpoint)
            bp = ttk.Frame(left)
            bp.pack(fill='x')
            self.bp = tk.StringVar()
            ttk.Entry(bp, textvariable=self.bp, width=12).pack(side='left', padx=3)
            ttk.Button(bp, text='Toggle breakpoint F9', command=self.breakpoint).pack(side='left')
            ttk.Label(bp, text='Case:').pack(side='left', padx=(8, 2))
            self.case_var = tk.StringVar()
            self.case_combo = ttk.Combobox(bp, textvariable=self.case_var, state='disabled', width=24)
            self.case_combo.pack(side='left', padx=2)
            self.case_combo.bind('<<ComboboxSelected>>', self.on_case_selected)
            self.case_bp_btn = ttk.Button(bp, text='BP @ Test', command=self.break_at_case_test, state='disabled')
            self.case_bp_btn.pack(side='left', padx=2)
            self.console = self.text(left, 'Guest serial output · control/binary bytes escaped', height=9)
            self.regs = ttk.Treeview(right, columns=('value', 'dec'), show='tree headings', height=10)
            self.regs.heading('#0', text='Register')
            self.regs.heading('value', text='Hex')
            self.regs.heading('dec', text='Decimal')
            self.regs.column('#0', width=65)
            self.regs.column('value', width=85)
            self.regs.column('dec', width=115)
            self.regs.tag_configure('changed', background='#fff0aa', foreground='#202020')
            self.regs.pack(fill='x')
            memory_bar = ttk.Frame(right)
            memory_bar.pack(fill='x')
            self.address = tk.StringVar(value='0x100')
            ttk.Entry(memory_bar, textvariable=self.address, width=10).pack(side='left')
            ttk.Button(memory_bar, text='View', command=lambda: self.send('view ' + self.address.get())).pack(side='left', padx=1)
            ttk.Button(memory_bar, text='0x100 Code', command=lambda: self.send('view 0x100')).pack(side='left', padx=1)
            ttk.Button(memory_bar, text='0x500 Data', command=lambda: self.send('view 0x500')).pack(side='left', padx=1)
            ttk.Button(memory_bar, text='Dump 32-bit Dec', command=lambda: self.send('mem32 ' + self.address.get() + ' 16 dec')).pack(side='left', padx=1)
            self.mem_cell_var = tk.StringVar(value='Affected cell: none')
            self.mem_cell_label = tk.Label(
                right, textvariable=self.mem_cell_var, anchor='w', justify='left',
                font=('Courier', 9, 'bold'), bg='#f1f5f9', fg='#0f172a', padx=6, pady=3, relief='groove',
            )
            self.mem_cell_label.pack(fill='x', pady=(2, 0))
            self.memory = self.text(right, 'Memory · physical RAM (affected cell highlighted in hex & ASCII)', height=4)
            self.stack = self.text(right, 'Stack · physical SS:SP/ESP view', height=3)
            for mem_widget in (self.memory, self.stack):
                mem_widget.tag_configure('changed_byte', background='#fef08a', foreground='#1e293b')
                mem_widget.tag_configure('affected_write', background='#facc15', foreground='#422006')
                mem_widget.tag_configure('affected_read', background='#7dd3fc', foreground='#082f49')
                mem_widget.tag_configure('affected_target', background='#fed7aa', foreground='#7c2d12')
            self.pil_view = self.text(right, 'PIL Translation · current instruction (Step PIL F8)', height=6)
            self.pil_view.tag_configure('pil_current', background='#d9eafa', foreground='#102030')
            self.pil_view.tag_configure('pil_done', background='#dcfce7', foreground='#14532d')
            self.inspector = self.text(right, 'Inspector · flags, segments and decode cache', height=4)
            bottom = ttk.Frame(root, padding=6)
            bottom.pack(fill='x')
            self.listener = self.text(bottom, 'Debugger listener · separate from guest output', height=5)
            row = ttk.Frame(bottom)
            row.pack(fill='x')
            self.command = tk.StringVar()
            entry = ttk.Entry(row, textvariable=self.command)
            entry.pack(side='left', fill='x', expand=True)
            entry.bind('<Return>', self.submit)
            entry.bind('<Up>', self.history_up)
            entry.bind('<Down>', self.history_down)
            ttk.Button(row, text='Send', command=self.submit).pack(side='left')
            ttk.Button(row, text='Clear log', command=lambda: self.replace(self.listener, '')).pack(side='left', padx=2)
            self.rx = tk.StringVar(value='41')
            ttk.Entry(row, textvariable=self.rx, width=13).pack(side='left', padx=4)
            ttk.Button(row, text='Guest RX hex', command=lambda: self.send('serial rx ' + self.rx.get())).pack(side='left')
            self.status = tk.StringVar(value='Connecting…')
            ttk.Label(root, textvariable=self.status, padding=6).pack(fill='x')
            for key, action in [('<F5>', self.run), ('<F6>', self.pause),
                                ('<F7>', lambda: self.send('step')),
                                ('<F8>', lambda: self.send('pilstep')),
                                ('<F9>', self.breakpoint)]:
                root.bind(key, lambda e, fn=action: fn())
            root.protocol('WM_DELETE_WINDOW', self.close)
            self.start(raw_path)
            root.after(20, self.poll)

        def text(self, parent, title, height):
            frame = ttk.LabelFrame(parent, text=title, padding=3)
            frame.pack(fill='both', expand=True, pady=3)
            widget = tk.Text(frame, height=height, wrap='none', font=('Courier', 10), state='disabled')
            scroll = ttk.Scrollbar(frame, orient='vertical', command=widget.yview)
            widget.configure(yscrollcommand=scroll.set)
            scroll.pack(side='right', fill='y')
            widget.pack(fill='both', expand=True)
            return widget

        def replace(self, widget, text):
            widget.configure(state='normal')
            widget.delete('1.0', 'end')
            widget.insert('1.0', text)
            widget.configure(state='disabled')

        def log(self, text):
            self.listener.configure(state='normal')
            self.listener.insert('end', text + '\n')
            if int(self.listener.index('end-1c').split('.')[0]) > 300:
                self.listener.delete('1.0', '100.0')
            self.listener.see('end')
            self.listener.configure(state='disabled')

        def _cleanup_temp(self):
            if self._temp_file:
                Path(self._temp_file).unlink(missing_ok=True)
                self._temp_file = None

        def start(self, raw_path, *, is_temp=False):
            if not is_temp:
                self._pending_case_index = None
                if raw_path:
                    p = Path(raw_path)
                    if p.suffix.lower() == '.json' or p.name.upper() == 'CPUTEST.COM':
                        manifest = load_case_manifest(p)
                        if manifest:
                            self.set_case_manifest(manifest)
                            self.load_case_index(0)
                            return
                        if p.suffix.lower() == '.json':
                            self.busy = False
                            msg = 'JSON file is not a valid DOSBox/PIL cases.json manifest'
                            self.status.set(msg)
                            messagebox.showerror('PIL debugger', msg, parent=root)
                            return
                self.set_case_manifest([])
            if self.client:
                self.client.close()
            if not is_temp:
                self._cleanup_temp()
            else:
                if self._temp_file and self._temp_file != str(raw_path):
                    Path(self._temp_file).unlink(missing_ok=True)
                self._temp_file = str(raw_path)
            self.running, self.busy, self.previous, self.prev_memory = False, True, {}, ''
            self.guest.clear()
            self.replace(self.console, '')
            try:
                self.client = EngineClient([self.engine], raw_path)
                self.status.set('Connecting…')
            except Exception as error:
                self.client = None
                self.busy = False
                self.status.set(str(error))
                messagebox.showerror('PIL debugger', str(error), parent=root)

        def set_case_manifest(self, cases):
            self.cases = list(cases)
            self._pending_case_index = None
            if self.cases:
                labels = [f"{c['index']:03d} {c['name']}" for c in self.cases]
                self.case_combo.configure(values=labels, state='readonly')
                self.case_bp_btn.configure(state='normal')
                self.log(f'Loaded {len(self.cases)} test cases from cases.json')
            else:
                self.case_combo.configure(values=[], state='disabled')
                self.case_var.set('')
                self.case_bp_btn.configure(state='disabled')
                self.active_case_meta = None

        def load_case_index(self, idx):
            if not (0 <= idx < len(self.cases)):
                return
            self._pending_case_index = None
            prev_meta = self.active_case_meta
            case = self.cases[idx]
            raw_bytes, meta = build_monitor_case(case)
            tmp = tempfile.NamedTemporaryFile(suffix='.bin', prefix='pil_case_', delete=False)
            try:
                tmp.write(raw_bytes)
                tmp.flush()
                tmp_path = tmp.name
            finally:
                tmp.close()
            self.active_case_meta = meta
            self.case_var.set(f"{meta['index']:03d} {meta['name']}")
            self.bp.set(f"0x{meta['test_eip']:x}")
            self.start(tmp_path, is_temp=True)
            if self.client is None:
                self.active_case_meta = prev_meta
                self.case_var.set(f"{prev_meta['index']:03d} {prev_meta['name']}" if prev_meta else '')
                return
            self.log(format_case_inspector(meta, meta['test_eip']))
            self.status.set(f"Loaded case #{meta['index']:03d} {meta['name']} (test @ 0x{meta['test_eip']:04x})")

        def on_case_selected(self, event=None):
            self.pause()
            idx = self.case_combo.current()
            if not (0 <= idx < len(self.cases)):
                return
            if self.busy:
                self._pending_case_index = idx
                case = self.cases[idx]
                self.status.set(f"Queued switch to case #{case['index']:03d} {case['name']}…")
                return
            self._pending_case_index = None
            self.load_case_index(idx)

        def on_demo_selected(self, event=None):
            self.pause()
            if self.send('demo ' + self.demo.get()):
                self._pending_case_index = None
                self.active_case_meta = None
                self.case_var.set('')

        def break_at_case_test(self):
            if not self.active_case_meta:
                return
            self.bp.set(f"0x{self.active_case_meta['test_eip']:x}")
            self.breakpoint()

        def send(self, command):
            if not self.client or self.busy or self.closing:
                self.status.set('Wait for the current command; Pause stops further run quanta.')
                return False
            try:
                if not self.client.request(command):
                    return False
            except (ValueError, RuntimeError) as error:
                self.status.set(str(error))
                return False
            self.busy = True
            return True

        def submit(self, event=None):
            self.pause()
            command = self.command.get().strip()
            if command and self.send(command):
                self.history.append(command)
                self.log('pil> ' + command)
                self.command.set('')

        def history_up(self, event=None):
            cmd = self.history.previous()
            if cmd is not None:
                self.command.set(cmd)
            return 'break'

        def history_down(self, event=None):
            cmd = self.history.next()
            if cmd is not None:
                self.command.set(cmd)
            return 'break'

        def run(self):
            if self.send('run 256'):
                self.running = True
                self.status.set('Running…')

        def pause(self):
            self.running = False
            self.status.set('Pausing at the end of the current quantum…' if self.busy else 'Paused')

        def breakpoint(self):
            self.pause()
            try:
                at = int(self.bp.get().strip(), 0) if self.bp.get().strip() else self.state['eip']
            except (ValueError, KeyError):
                self.status.set('Enter an address such as0x100')
                return
            self.send(('delete' if at in self.state.get('breakpoints', []) else 'break') + f' 0x{at:x}')

        def code_breakpoint(self, event):
            line = self.code.get(f'@{event.x},{event.y} linestart', f'@{event.x},{event.y} lineend')
            try:
                at = int(line.lstrip('* ').split()[0], 16)
            except (ValueError, IndexError):
                return
            self.bp.set(f'0x{at:x}')
            self.breakpoint()

        def open_file(self):
            self.pause()
            if self.busy:
                return
            initial = self._last_dir if Path(self._last_dir).is_dir() else None
            name = filedialog.askopenfilename(
                parent=root,
                title='Open program (Hex, Binary, or Image)',
                initialdir=initial,
                filetypes=[
                    ('Executable, Hex & Case manifests', '*.hex *.bin *.com *.json'),
                    ('Hex programs (*.hex)', '*.hex'),
                    ('Raw binary (*.bin)', '*.bin'),
                    ('DOS COM (*.com)', '*.com'),
                    ('Case manifest (*.json)', '*.json'),
                    ('Disk images & archives (*.img *.ima *.zip)', '*.img *.ima *.zip'),
                    ('All files (*.*)', '*.*'),
                ],
            )
            if not name:
                return
            self._last_dir = str(Path(name).parent)
            path = Path(name)
            ext = path.suffix.lower()
            if ext in {'.img', '.ima', '.zip'}:
                self.open_image_browser(name)
                return
            try:
                if ext == '.json':
                    manifest = load_case_manifest(path)
                    if not manifest:
                        raise ValueError('JSON file is not a valid DOSBox/PIL cases.json manifest')
                    self.set_case_manifest(manifest)
                    self.load_case_index(0)
                    return

                if ext == '.hex':
                    raw_bytes = read_bounded_hex_file(path)
                    if not 1 <= len(raw_bytes) <= MAX_MONITOR_BYTES:
                        raise ValueError(f'Program contains {len(raw_bytes)} bytes; monitor limit is 1..{MAX_MONITOR_BYTES} bytes')
                    tmp = tempfile.NamedTemporaryFile(suffix='.bin', prefix='pil_hex_', delete=False)
                    try:
                        tmp.write(raw_bytes)
                        tmp.flush()
                        tmp_path = tmp.name
                    finally:
                        tmp.close()
                    self.set_case_manifest([])
                    self.start(tmp_path, is_temp=True)
                    self.status.set(f'Loaded hex program: {path.name} ({len(raw_bytes)} bytes)')
                    return

                size = path.stat().st_size
                if path.name.upper() == 'CPUTEST.COM':
                    manifest = load_case_manifest(path)
                    if manifest:
                        self.set_case_manifest(manifest)
                        self.load_case_index(0)
                        return
                if not 1 <= size <= MAX_MONITOR_BYTES:
                    raise ValueError(f'Raw monitor images must contain 1..{MAX_MONITOR_BYTES} bytes; use --dos-probe for DOS executables ({size} bytes)')
                self.set_case_manifest([])
                self.start(str(path))
            except (OSError, ValueError) as error:
                messagebox.showerror('Open program failed', str(error), parent=root)

        def open_image_dialog(self):
            self.pause()
            if self.busy:
                return
            initial = self._last_dir if Path(self._last_dir).is_dir() else None
            name = filedialog.askopenfilename(
                parent=root,
                title='Open disk image or archive',
                initialdir=initial,
                filetypes=[
                    ('Disk images & archives (*.img *.ima *.zip)', '*.img *.ima *.zip'),
                    ('All files (*.*)', '*.*'),
                ],
            )
            if name:
                self._last_dir = str(Path(name).parent)
                self.open_image_browser(name)

        def open_image_browser(self, image_path):
            img_path = Path(image_path)
            entries = []
            entry_sizes = {}

            win = tk.Toplevel(root)
            win.title(f'Container Browser — {img_path.name}')
            win.geometry('720x500')
            win.minsize(500, 320)

            top = ttk.Frame(win, padding=6)
            top.pack(fill='x')
            ttk.Label(top, text='Filter:').pack(side='left', padx=(0, 4))
            filter_var = tk.StringVar()
            filter_entry = ttk.Entry(top, textvariable=filter_var)
            filter_entry.pack(side='left', fill='x', expand=True)
            browser_status = tk.StringVar(value=f'Reading {img_path.name}…')
            ttk.Label(top, textvariable=browser_status).pack(side='right', padx=(6, 0))

            tree_frame = ttk.Frame(win, padding=6)
            tree_frame.pack(fill='both', expand=True)
            tree = ttk.Treeview(tree_frame, columns=('size', 'kind'), show='tree headings')
            tree.heading('#0', text='File Path')
            tree.heading('size', text='Size (B)')
            tree.heading('kind', text='Type')
            tree.column('#0', width=450)
            tree.column('size', width=100, anchor='e')
            tree.column('kind', width=70, anchor='center')

            v_scroll = ttk.Scrollbar(tree_frame, orient='vertical', command=tree.yview)
            tree.configure(yscrollcommand=v_scroll.set)
            v_scroll.pack(side='right', fill='y')
            tree.pack(fill='both', expand=True)

            def populate():
                if not win.winfo_exists():
                    return
                tree.delete(*tree.get_children())
                q = filter_var.get().strip().lower()
                for ent_path, size, kind in entries:
                    if not q or q in ent_path.lower():
                        tree.insert('', 'end', iid=ent_path, text=ent_path, values=(f'{size:,}', kind))

            filter_var.trace_add('write', lambda *_: populate())

            btn_frame = ttk.Frame(win, padding=6)
            btn_frame.pack(fill='x')

            def get_selected():
                sel = tree.selection()
                if not sel:
                    messagebox.showinfo('Selection', 'Select a file inside the container first.', parent=win)
                    return None
                return sel[0]

            def load_to_monitor():
                sel = get_selected()
                if not sel:
                    return
                known_size = entry_sizes.get(sel)
                if known_size is not None and not 1 <= known_size <= MAX_MONITOR_BYTES:
                    messagebox.showerror(
                        'Load Error',
                        f'File contains {known_size} bytes (monitor max is {MAX_MONITOR_BYTES:,} bytes). Use Run DOS Probe instead.',
                        parent=win,
                    )
                    return

                requested_client = self.client

                def work():
                    tmp = tempfile.NamedTemporaryFile(suffix='.bin', prefix='pil_ent_', delete=False)
                    tmp.close()
                    try:
                        subprocess.run(
                            [str(self.engine), '--dos-read', str(img_path), sel, tmp.name],
                            capture_output=True, text=True, check=True, timeout=10,
                        )
                        size = Path(tmp.name).stat().st_size
                        if not 1 <= size <= MAX_MONITOR_BYTES:
                            raise ValueError(f'File contains {size} bytes (monitor max is {MAX_MONITOR_BYTES:,} bytes). Use Run DOS Probe instead.')
                        return tmp.name, size
                    except Exception:
                        Path(tmp.name).unlink(missing_ok=True)
                        raise

                def load(tmp_name, size):
                    self.set_case_manifest([])
                    self.start(tmp_name, is_temp=True)
                    self.status.set(f'Loaded {sel} ({size} bytes) from {img_path.name}')
                    win.destroy()

                def on_done(res, error):
                    complete_monitor_load(
                        res, error,
                        is_current=lambda: not self.closing and self.client is requested_client and bool(win.winfo_exists()),
                        load=load,
                        report_error=lambda exc: messagebox.showerror('Load Error', str(exc), parent=win),
                    )

                if not self.jobs.submit(work, on_done):
                    self.status.set('Another container task is already running…')

            def run_probe():
                sel = get_selected()
                if not sel:
                    return
                if win.winfo_exists():
                    win.destroy()
                self.log(f'pil> dosprobe {img_path}::{sel}')
                self.status.set(f'Running DOS probe on {sel}…')

                def work():
                    res = subprocess.run(
                        [str(self.engine), '--dos-probe', f'{img_path}::{sel}'],
                        capture_output=True, text=True, check=True, timeout=15,
                    )
                    return res.stdout

                def on_done(stdout, error):
                    if error is not None:
                        self.log(f'DOS probe error: {error}')
                        self.status.set(f'DOS probe error: {error}')
                        return
                    for line in stdout.splitlines():
                        self.log(line)
                    self.status.set(f'DOS probe finished for {sel}')

                if not self.jobs.submit(work, on_done):
                    self.status.set('Another container task is already running…')

            def extract_file():
                sel = get_selected()
                if not sel:
                    return
                dest = filedialog.asksaveasfilename(parent=win, title='Extract file to…', initialfile=Path(sel).name)
                if not dest:
                    return

                def work():
                    subprocess.run(
                        [str(self.engine), '--dos-read', str(img_path), sel, dest],
                        capture_output=True, text=True, check=True, timeout=10,
                    )
                    return dest

                def on_done(out_dest, error):
                    parent = win if win.winfo_exists() else root
                    if error is not None:
                        messagebox.showerror('Extract failed', str(error), parent=parent)
                    else:
                        messagebox.showinfo('Extracted', f'Successfully extracted to {out_dest}', parent=parent)

                if not self.jobs.submit(work, on_done):
                    self.status.set('Another container task is already running…')

            action_buttons = [
                ttk.Button(btn_frame, text='Load into Monitor', command=load_to_monitor, state='disabled'),
                ttk.Button(btn_frame, text='Run DOS Probe', command=run_probe, state='disabled'),
                ttk.Button(btn_frame, text='Extract…', command=extract_file, state='disabled'),
            ]
            for btn in action_buttons:
                btn.pack(side='left', padx=3)
            ttk.Button(btn_frame, text='Close', command=win.destroy).pack(side='right', padx=3)
            tree.bind('<Double-Button-1>', lambda e: load_to_monitor())

            def list_work():
                proc = subprocess.run(
                    [str(self.engine), '--dos-list', '--all', str(img_path)],
                    capture_output=True, text=True, timeout=10,
                )
                if proc.returncode != 0:
                    proc = subprocess.run(
                        [str(self.engine), '--dos-list', str(img_path)],
                        capture_output=True, text=True, timeout=10,
                    )
                if proc.returncode != 0:
                    raise RuntimeError(proc.stderr.strip() or proc.stdout.strip() or f'Exit status {proc.returncode}')
                return parse_container_listing(proc.stdout)

            def on_listed(parsed, error):
                if error is not None:
                    if win.winfo_exists():
                        win.destroy()
                    messagebox.showerror('Image Browser', f'Failed to read container {img_path.name}:\n{error}', parent=root)
                    return
                entries.clear()
                entries.extend(parsed)
                entry_sizes.clear()
                for ent_path, size, _ in parsed:
                    entry_sizes[ent_path] = size
                if win.winfo_exists():
                    browser_status.set(f'{len(entries)} entries')
                    for btn in action_buttons:
                        btn.configure(state='normal')
                    populate()

            if not self.jobs.submit(list_work, on_listed):
                browser_status.set('Busy')

        def render(self, command, message):
            self.state = state = message['state']
            root.title(f"PIL {state['version']} · {state['demo']}")
            values = dict(zip(['EAX', 'ECX', 'EDX', 'EBX', 'ESP', 'EBP', 'ESI', 'EDI'], state['registers']))
            values.update(EIP=state['eip'], EFLAGS=state['flags'])
            self.regs.delete(*self.regs.get_children())
            for name, value in values.items():
                changed = name in self.previous and self.previous[name] != value
                self.regs.insert('', 'end', text=name, values=(f'{value:08X}', f'{value:,}'), tags=('changed',) if changed else ())
            self.previous = values
            self.replace(self.code, state['listing'])
            for row, line in enumerate(state['listing'].splitlines(), 1):
                try:
                    at = int(line.lstrip('* ').split()[0], 16)
                except (ValueError, IndexError):
                    continue
                if at == state['eip']:
                    self.code.tag_add('current', f'{row}.0', f'{row}.end')
                    self.code.see(f'{row}.0')
                if at in state['breakpoints']:
                    self.code.tag_add('breakpoint', f'{row}.0', f'{row}.end')
            self.replace(self.memory, state['memory'])
            self.replace(self.stack, state['stack'])
            effect = state.get('memory_effect')
            self.mem_cell_var.set(format_memory_cell_value(effect))
            if isinstance(effect, dict):
                stage = effect.get('stage', '')
                if stage == 'written':
                    cell_tag, banner_bg, banner_fg = 'affected_write', '#fef08a', '#422006'
                elif stage == 'read':
                    cell_tag, banner_bg, banner_fg = 'affected_read', '#e0f2fe', '#082f49'
                else:
                    cell_tag, banner_bg, banner_fg = 'affected_target', '#ffedd5', '#7c2d12'
                self.mem_cell_label.configure(bg=banner_bg, fg=banner_fg)
            else:
                cell_tag = None
                self.mem_cell_label.configure(bg='#f1f5f9', fg='#475569')
            if self.prev_memory and not (message['ok'] and (command == 'reset' or command.startswith('demo '))):
                for s_idx, e_idx in find_changed_memory_spans(self.prev_memory, state['memory']):
                    self.memory.tag_add('changed_byte', s_idx, e_idx)
            self.prev_memory = state['memory']
            if cell_tag:
                mem_spans = find_memory_highlight_spans(state['memory'], effect)
                for s_idx, e_idx in mem_spans:
                    self.memory.tag_add(cell_tag, s_idx, e_idx)
                if mem_spans:
                    self.memory.see(mem_spans[0][0])
                for s_idx, e_idx in find_memory_highlight_spans(state['stack'], effect):
                    self.stack.tag_add(cell_tag, s_idx, e_idx)
            if 'pil' in state:
                self.replace(self.pil_view, state['pil'])
                for row, line in enumerate(state['pil'].splitlines(), 1):
                    if line.startswith('> '):
                        self.pil_view.tag_add('pil_current', f'{row}.0', f'{row}.end')
                        self.pil_view.see(f'{row}.0')
                    elif line.startswith('✓ '):
                        self.pil_view.tag_add('pil_done', f'{row}.0', f'{row}.end')
            if message['ok'] and (command.startswith('demo ') or command.startswith('load ')):
                self.active_case_meta = None
                self.case_var.set('')
            case_line = format_case_inspector(self.active_case_meta, state['eip'])
            inspector_text = (
                (case_line + '\n' if case_line else '')
                + f"Undefined flags={state['undefined_flags']:08X}\n{state['segments']}\nBreakpoints: "
                + ', '.join(f'{n:08x}' for n in state['breakpoints']) + '\n' + state['cache']
            )
            self.replace(self.inspector, inspector_text)
            self.address.set(f"0x{state['memory_address']:x}")
            if message['ok'] and (command == 'reset' or command.startswith('demo ')):
                self.guest.clear()
            self.guest.extend(message['guest_output'])
            del self.guest[:-65536]
            shown = ''.join(chr(b) if b in (9, 10) or 32 <= b <= 126 else f'\\x{b:02x}' for b in self.guest)
            self.replace(self.console, shown)
            self.console.see('end')
            if message['text'] and (command != 'run 256' or message['status'] not in {'budget', 'stepped'}):
                self.log(message['text'])
            self.status.set(message['status'].replace('_', ' ') + (': ' + message['text'].split('\n')[0] if message['text'] else ''))

        def poll(self):
            if self.closing:
                return
            self.jobs.drain()
            if self.client:
                try:
                    event, command, value = self.client.events.get_nowait()
                except queue.Empty:
                    pass
                else:
                    self.busy = False
                    if self._pending_case_index is not None and not self.closing:
                        pending_idx = self._pending_case_index
                        self._pending_case_index = None
                        self.load_case_index(pending_idx)
                    elif event == 'error':
                        self.running = False
                        self.log('Debugger disconnected: ' + value)
                        self.status.set('Disconnected')
                    else:
                        self.render(command, value)
                        if value['quit']:
                            self.status.set('Debugger session ended')
                        action = next_command(self.running, value)
                        if action:
                            root.after(1, lambda c=action: self.send(c) if self.running else None)
                        else:
                            self.running = False
            root.after(20, self.poll)

        def close(self):
            self.closing, self.running = True, False
            if self.client:
                self.client.close()
            self._cleanup_temp()
            root.destroy()

    return App()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', help='path to built pil-proof executable')
    parser.add_argument('--load', metavar='RAW_BINARY', help='raw monitor image; DOS loading is not yet in this window')
    args = parser.parse_args(argv)
    try:
        engine = find_engine(args.engine)
        import tkinter as tk
        try:
            root = tk.Tk()
        except tk.TclError as error:
            parser.exit(1, f'PIL debugger needs a graphical display: {error}\n')
        make_app(root, engine, args.load)
        root.mainloop()
    except (ImportError, OSError, RuntimeError) as error:
        parser.exit(1, f'PIL debugger: {error}\n')


if __name__ == '__main__':
    main()
