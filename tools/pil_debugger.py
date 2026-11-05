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
STATUSES = {'ready', 'stepped', 'budget', 'breakpoint', 'waiting_rx', 'tx_blocked',
            'halt', 'fault', 'decode', 'missing_instruction', 'error'}


class ProtocolError(RuntimeError):
    pass


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
            or not all(map(integer, state['breakpoints']))):
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
    for root in [Path(__file__).resolve().parent, Path(__file__).resolve().parent.parent]:
        for path in [root / name, root / 'target/release' / name, root / 'target/debug' / name]:
            if path.is_file():
                return path
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
            root.title('PIL debugger')
            root.geometry('1180x780')
            root.minsize(850, 560)
            style = ttk.Style(root)
            if 'clam' in style.theme_names():
                style.theme_use('clam')
            bar = ttk.Frame(root, padding=6)
            bar.pack(fill='x')
            for label, callback in [('Open raw binary…', self.open_file), ('Step F7', lambda: self.send('step')),
                                    ('Run F5', self.run), ('Pause F6', self.pause),
                                    ('Reset', lambda: self.send('reset'))]:
                ttk.Button(bar, text=label, command=callback).pack(side='left', padx=3)
            self.demo = tk.StringVar(value='counter')
            chooser = ttk.Combobox(bar, textvariable=self.demo, state='readonly', width=14,
                                  values=['counter', 'echo', 'memory', 'selftest', 'hello',
                                          'factorials', 'loop', 'mandelbrot', 'extensions'])
            chooser.pack(side='right')
            chooser.bind('<<ComboboxSelected>>', lambda e: self.send('demo ' + self.demo.get()))
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
            ttk.Entry(bp, textvariable=self.bp, width=15).pack(side='left', padx=3)
            ttk.Button(bp, text='Toggle breakpoint F9', command=self.breakpoint).pack(side='left')
            self.console = self.text(left, 'Guest serial output · control/binary bytes escaped', height=9)
            self.regs = ttk.Treeview(right, columns=('value',), show='tree headings', height=10)
            self.regs.heading('#0', text='Register')
            self.regs.heading('value', text='Hex value')
            self.regs.column('#0', width=95)
            self.regs.column('value', width=140)
            self.regs.tag_configure('changed', background='#fff0aa', foreground='#202020')
            self.regs.pack(fill='x')
            memory_bar = ttk.Frame(right)
            memory_bar.pack(fill='x')
            self.address = tk.StringVar(value='0x100')
            ttk.Entry(memory_bar, textvariable=self.address, width=16).pack(side='left')
            ttk.Button(memory_bar, text='View memory', command=lambda: self.send('view ' + self.address.get())).pack(side='left')
            self.memory = self.text(right, 'Memory · physical RAM', height=5)
            self.stack = self.text(right, 'Stack · physical SS:SP/ESP view', height=3)
            self.inspector = self.text(right, 'Inspector · flags, segments and decode cache', height=7)
            bottom = ttk.Frame(root, padding=6)
            bottom.pack(fill='x')
            self.listener = self.text(bottom, 'Debugger listener · separate from guest output', height=5)
            row = ttk.Frame(bottom)
            row.pack(fill='x')
            self.command = tk.StringVar()
            entry = ttk.Entry(row, textvariable=self.command)
            entry.pack(side='left', fill='x', expand=True)
            entry.bind('<Return>', self.submit)
            ttk.Button(row, text='Send', command=self.submit).pack(side='left')
            self.rx = tk.StringVar(value='41')
            ttk.Entry(row, textvariable=self.rx, width=13).pack(side='left', padx=4)
            ttk.Button(row, text='Guest RX hex', command=lambda: self.send('serial rx ' + self.rx.get())).pack(side='left')
            self.status = tk.StringVar(value='Connecting…')
            ttk.Label(root, textvariable=self.status, padding=6).pack(fill='x')
            for key, action in [('<F5>', self.run), ('<F6>', self.pause),
                                ('<F7>', lambda: self.send('step')), ('<F9>', self.breakpoint)]:
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

        def start(self, raw_path):
            if self.client:
                self.client.close()
            self.running, self.busy, self.previous = False, True, {}
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
                self.log('pil> ' + command)
                self.command.set('')

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
            name = filedialog.askopenfilename(parent=root, title='Open raw monitor binary (not a DOS COM/MZ loader)')
            if name:
                try:
                    size = Path(name).stat().st_size
                    if not 1 <= size <= 3840:
                        raise ValueError('Raw monitor images must contain1..3840 bytes; use --dos-probe for DOS executables/images')
                except (OSError, ValueError) as error:
                    messagebox.showerror('PIL raw image', str(error), parent=root)
                    return
                self.start(name)

        def render(self, command, message):
            self.state = state = message['state']
            root.title(f"PIL {state['version']} · {state['demo']}")
            values = dict(zip(['EAX', 'ECX', 'EDX', 'EBX', 'ESP', 'EBP', 'ESI', 'EDI'], state['registers']))
            values.update(EIP=state['eip'], EFLAGS=state['flags'])
            self.regs.delete(*self.regs.get_children())
            for name, value in values.items():
                changed = name in self.previous and self.previous[name] != value
                self.regs.insert('', 'end', text=name, values=(f'{value:08X}',), tags=('changed',) if changed else ())
            self.previous = values
            self.replace(self.code, state['listing'])
            for row, line in enumerate(state['listing'].splitlines(), 1):
                try:
                    at = int(line.lstrip('* ').split()[0], 16)
                except (ValueError, IndexError):
                    continue
                if at == state['eip']:
                    self.code.tag_add('current', f'{row}.0', f'{row}.end')
                if at in state['breakpoints']:
                    self.code.tag_add('breakpoint', f'{row}.0', f'{row}.end')
            self.replace(self.memory, state['memory'])
            self.replace(self.stack, state['stack'])
            self.replace(self.inspector, f"Undefined flags={state['undefined_flags']:08X}\n{state['segments']}\nBreakpoints: "
                         + ', '.join(f'{n:08x}' for n in state['breakpoints']) + '\n' + state['cache'])
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
            if self.client:
                try:
                    event, command, value = self.client.events.get_nowait()
                except queue.Empty:
                    pass
                else:
                    self.busy = False
                    if event == 'error':
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
