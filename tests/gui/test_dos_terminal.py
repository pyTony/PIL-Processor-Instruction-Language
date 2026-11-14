import importlib.util
import io
import json
from types import SimpleNamespace
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch, MagicMock
spec = importlib.util.spec_from_file_location('dos_terminal', Path(__file__).resolve().parents[2] / 'tools/dos_terminal.py')
term = importlib.util.module_from_spec(spec)
spec.loader.exec_module(term)

# MOV AH,9; MOV DX,msg; INT 21; loop: MOV AH,1; INT 21; CMP AL,0D; JNE loop; MOV AX,4C00; INT 21; NOP; "Name? $"
ASK = bytes([0xb4, 0x09, 0xba, 0x15, 0x01, 0xcd, 0x21, 0xb4, 0x01, 0xcd, 0x21, 0x3c, 0x0d, 0x75, 0xf8,
             0xb8, 0x00, 0x4c, 0xcd, 0x21, 0x90]) + b'Name? $'
# Writes its command tail (PSP:81h, length at 80h) to standard output and exits.
TAIL = bytes([0xb4, 0x40, 0xbb, 0x01, 0x00, 0x32, 0xed, 0x8a, 0x0e, 0x80, 0x00, 0xba, 0x81, 0x00, 0xcd, 0x21,
              0xcd, 0x20])


class DosTerminalTests(unittest.TestCase):

    def test_native_graphics_child_environment(self):
        frames = [dict(protocol=1, seq=0, ok=True, native_graphics=True)]
        def process():
            return SimpleNamespace(stdin=io.StringIO(), stdout=io.StringIO(
                ''.join(json.dumps(v) + '\n' for v in frames)), stderr=io.StringIO())
        with patch.object(term.subprocess, 'Popen', return_value=process()) as popen:
            term.Session('engine', 'program', rust_graphics=True)
            self.assertEqual(popen.call_args.kwargs['env']['DINE_VIDEO'], 'window')
        with patch.object(term.subprocess, 'Popen', return_value=process()) as popen:
            term.Session('engine', 'program')
            self.assertNotIn('env', popen.call_args.kwargs)

    def test_native_bios_key_conversion(self):
        self.assertEqual(term.bios_keys(b'a\r\x1b\x08\t'),
                         [(97, 30), (13, 28), (27, 1), (8, 14), (9, 15)])
        import queue
        port = term.ConsolePort.__new__(term.ConsolePort)
        port.input = queue.Queue()
        port.input.put((0, 72))
        port.input.put(b'Z')
        self.assertEqual(port.read_bios(), [(0, 72), (90, 44)])

    def test_native_bios_key_not_com1(self):
        class Session:
            com1_out = b''
            last = {}
            def __init__(self): self.commands = []
            def send(self, command):
                self.commands.append(command)
                if command.startswith('run '):
                    return {'status': 'exit' if 'key 97 30' in self.commands else 'waiting_input',
                            'native_graphics': True}
                return {'status': 'waiting_input', 'native_graphics': True}
        class Port:
            def __init__(self): self.calls = 0
            def read_bios(self):
                self.calls += 1
                return [(97, 30)] if self.calls == 1 else []
            def read(self): raise AssertionError('graphics BIOS key reached COM1')
        session=Session()
        with patch.object(term, 'HeadlessGraphics') as graphics:
            result=term.run_headless(session, Port(), 0, is_shell=False, rust_graphics=True)
            graphics.assert_not_called()
        self.assertEqual(result['status'], 'exit')
        self.assertIn('key 97 30', session.commands)

    def test_native_window_close_is_distinct_from_dos_exit(self):
        class Session:
            com1_out = b''
            last = {}
            def send(self, command):
                return ({'status': 'waiting_input', 'native_graphics': True,
                         'native_window_closed': True} if command.startswith('run ')
                        else {'status': 'running', 'native_graphics': True})
        class Port:
            def read(self): return b''
        self.assertEqual(term.run_headless(Session(), Port(), 0, is_shell=False,
                                            rust_graphics=True)['status'], 'window_closed')

    def test_rust_graphics_option_requires_headless(self):
        with self.assertRaises(SystemExit):
            term.main(['--rust-graphics', 'program.com'])


    def test_mouse_opt_in_sends_command_before_guest_execution(self):
        frames = ''.join(json.dumps({'protocol': 1, 'seq': i, 'ok': True}) + '\n' for i in range(2))
        process = SimpleNamespace(stdin=io.StringIO(), stdout=io.StringIO(frames), stderr=io.StringIO())
        with patch.object(term.subprocess, 'Popen', return_value=process):
            session = term.Session('engine', 'program', mouse=True)
        self.assertEqual(process.stdin.getvalue(), 'mouse on\n')
        self.assertEqual(session.seq, 1)

    def test_dos_mouse_session_click_reaches_int33_registers(self):
        # MOV AX,0 / INT33 reset / MOV AX,3 / INT33 poll / INT20 exit
        code = bytes.fromhex('b8 00 00 cd 33 b8 03 00 cd 33 cd 20')
        engine = term.find_engine()
        with tempfile.TemporaryDirectory() as directory:
            program = Path(directory) / 'mouse.com'
            program.write_bytes(code)
            session = term.Session(engine, program, mouse=True, stop_log='')
            try:
                reply = session.send('run 2')
                self.assertTrue(reply['mouse_active'])
                self.assertTrue(reply['mouse_present'])
                reply = session.send('mouse 160 80 1')
                self.assertTrue(reply['ok'])
                reply = session.send('run 32')
                self.assertEqual(reply['status'], 'exit')
                regs = reply['registers']
                self.assertEqual((regs[3] & 0xffff, regs[1] & 0xffff, regs[2] & 0xffff),
                                 (1, 160, 80))
            finally:
                session.close()

    def test_dos_mouse_coordinates_buttons_and_overlay(self):
        self.assertEqual(term.mouse_virtual_coords(320, 240, 640, 480, 639, 199), (320, 100))
        self.assertEqual(term.mouse_virtual_coords(-10, 999, 640, 480, 639, 479), (0, 479))
        self.assertEqual(term.mouse_virtual_coords(639, 479, 640, 480, 639, 479), (639, 479))
        self.assertEqual(term.mouse_command(80, 16, 3), 'mouse 80 16 3')
        events = term.MouseEvents()
        for x, buttons in ((10, 0), (20, 1), (30, 1), (40, 1), (50, 0)):
            events.push(x, 4, buttons)
        self.assertEqual(events.pending, [(10, 4, 0), (20, 4, 1), (40, 4, 1), (50, 4, 0)])
        rgb = bytes([127] * (20 * 20 * 3))
        painted = term.mouse_overlay(rgb, 20, 20, 0, 0, 639, 199)
        self.assertEqual(rgb, bytes([127] * (20 * 20 * 3)))
        self.assertEqual(painted[:3], bytes(3))
        self.assertNotEqual(painted, rgb)

    def test_program_defined_graphics_cursor_masks(self):
        # Row 0: screen 0x7FFF, cursor 0x8000 -> pixel 0 white, others kept.
        # Row 1: screen 0x3FFF, cursor 0x4000 -> pixel 0 black, pixel 1 white.
        # Row 2: screen 0xFFFF, cursor 0x8000 -> pixel 0 inverted.
        screen = [0x7fff, 0x3fff, 0xffff] + [0xffff] * 13
        cursor = [0x8000, 0x4000, 0x8000] + [0] * 13
        shape = ''.join(f'{w & 255:02x}{w >> 8:02x}' for w in screen + cursor)
        rgb = bytes([100] * (8 * 8 * 3))
        # Hot spot (1,1) at the point (1,1) puts the bitmap's corner at (0,0).
        painted = term.mouse_overlay(rgb, 8, 8, 1, 1, 7, 7, shape, (1, 1))
        pixel = lambda x, y: painted[(y * 8 + x) * 3:(y * 8 + x) * 3 + 3]
        self.assertEqual(pixel(0, 0), b'\xff\xff\xff')
        self.assertEqual(pixel(1, 0), bytes([100] * 3))
        self.assertEqual(pixel(0, 1), bytes(3))
        self.assertEqual(pixel(1, 1), b'\xff\xff\xff')
        self.assertEqual(pixel(0, 2), bytes([155] * 3))
        self.assertEqual(rgb, bytes([100] * (8 * 8 * 3)))

    def test_native_statistics_separate_blocks_singles_and_services(self):
        output = term.format_native_stats({
            'steps': 1000, 'native_blocks': 100,
            'native_block_instructions': 400,
            'native_single_instructions': 300,
        })
        self.assertIn('Native total: 700 (70.0% of steps)', output)
        self.assertIn('Native single instructions: 300', output)
        self.assertIn('Other steps: 300 (includes DOS/BIOS services)', output)
        self.assertIn('Average native block length: 4.00', output)
        self.assertIn('stats unavailable', term.format_native_stats({'steps': 10}))
        self.assertIn('n/a', term.format_native_stats({
            'steps': 0, 'native_blocks': 0,
            'native_block_instructions': 0, 'native_single_instructions': 0,
        }))


    def test_raw_key_release_does_not_block_later_keys_for_modifier_settle(self):
        # The keydown must reach the guest before its keyup, but no further
        # ten idle ticks should hold that release and every following key.
        self.assertTrue(term.defer_key_command('keyup 97 30', sent=True, settle=10))
        self.assertFalse(term.defer_key_command('keyup 97 30', sent=False, settle=10))
        # Alt/Ctrl/Shift state changes retain the guest run-slice delay used
        # by Turbo and EDIT's function-key/modifier combinations.
        self.assertTrue(term.defer_key_command('shift 0', sent=True, settle=0))
        self.assertTrue(term.defer_key_command('shift 0', sent=False, settle=10))
        self.assertFalse(term.defer_key_command('shift 0', sent=False, settle=0))
        self.assertFalse(term.defer_key_command('keydown 97 30', sent=True, settle=10))
        self.assertFalse(term.defer_key_command('key 97 30', sent=True, settle=10))

    def test_queued_keyboard_input_keeps_tk_tick_fast_while_guest_is_idle(self):
        self.assertEqual(term.next_tick_delay(True, 'waiting_input', True), 1)
        self.assertEqual(term.next_tick_delay(True, 'running', True), 1)
        self.assertEqual(term.next_tick_delay(False, 'waiting_input', True), 40)
        self.assertEqual(term.next_tick_delay(False, 'running', True), 40)
        self.assertEqual(term.next_tick_delay(False, 'running', False), 1)


    def test_partial_80387_mode_is_explicit_and_reports_bios_equipment(self):
        engine = term.find_engine()
        with tempfile.TemporaryDirectory() as directory:
            program = Path(directory) / 'fpu.com'
            program.write_bytes(bytes.fromhex('dbe3 d9e8 cd11 cd20'))
            for mode, present in [('none', False), ('80387', True)]:
                session = term.Session(engine, program, fpu=mode)
                try:
                    self.assertIs(session.last['fpu_present'], present)
                    reply = session.send('run 16')
                    self.assertEqual(reply['status'], 'exit')
                    self.assertEqual(reply['registers'][0] & 2, int(present) * 2)
                finally:
                    session.close()

    def test_buffered_and_full_frames_decode_cp437_consistently(self):
        engine = term.find_engine()
        with tempfile.TemporaryDirectory() as directory:
            program = Path(directory) / 'cp437.com'
            program.write_bytes(bytes.fromhex('b402b284cd21cd20'))
            for command in ['run 100', 'run 100 buffered']:
                session = term.Session(engine, program, stop_log='')
                try:
                    reply = session.send(command)
                    self.assertEqual(reply['status'], 'exit')
                    self.assertEqual(reply['output'], 'ä')
                finally:
                    session.close()

    def test_cga_mode_control_blanks_and_selects_blink_or_bright_background(self):
        reply = dict(screen='418f', columns=1, text_blink=True, video_enabled=True)
        self.assertEqual(term.screen_rows(reply, blink_on=True), [('A', [15])])
        self.assertEqual(term.screen_rows(reply, blink_on=False), [(' ', [15])])
        reply['text_blink'] = False
        self.assertEqual(term.screen_rows(reply, blink_on=False), [('A', [143])])
        reply['video_enabled'] = False
        self.assertEqual(term.screen_rows(reply), [(' ', [0])])
        self.assertEqual(reply['screen'], '418f')

    def test_buffered_reply_keeps_frame_until_snapshot_and_clears_old_mode(self):
        frames = [dict(protocol=1, seq=0, ok=True, buffered_run=True, graphics13='00', palette='11'),
                  dict(protocol=1, seq=1, ok=True, frame_pending=True, status='waiting_input', com1='58'),
                  dict(protocol=1, seq=2, ok=True, buffered_run=True, status='exit', screen='2007')]
        process = SimpleNamespace(stdin=io.StringIO(), stdout=io.StringIO(''.join(json.dumps(f)+'\n' for f in frames)), stderr=io.StringIO())
        with patch.object(term.subprocess, 'Popen', return_value=process):
            session = term.Session('engine', 'program')
            reply = term.run_slice(session, 100)
            self.assertEqual(reply['graphics13'], '00')
            self.assertEqual(reply['status'], 'waiting_input')
            reply = term.refresh_frame(session, reply)
            self.assertNotIn('graphics13', reply)
            self.assertNotIn('frame_pending', reply)
            self.assertEqual(session.com1_out, b'X')
        self.assertEqual(process.stdin.getvalue(), 'run 100 buffered\nsnapshot\n')

    def test_headless_auto_key_only_at_graphics_wait(self):
        class Session:
            def __init__(self):
                self.com1_out = b''
                self.key = False
                self.commands = []
            def send(self, command):
                self.commands.append(command)
                if command == 'key 13 28': self.key = True
                if command.startswith('run '):
                    return {'status': 'exit' if self.key else 'waiting_input', 'ok': True, 'graphics_cga': '00'}
                return {'status': 'running', 'ok': True}
        session = Session(); port = MagicMock(); port.read.return_value = b''
        with patch.object(term, 'HeadlessGraphics') as graphics:
            graphics.return_value.stop_requested = False
            result = term.run_headless(session, port, 0, auto_graphics_key=True)
        self.assertEqual(result['status'], 'exit')
        self.assertEqual(session.commands.count('key 13 28'), 1)
        self.assertNotIn('key 99 46', session.commands)  # no CTTY injection

    def test_headless_batches_busy_slices_before_redrawing(self):
        class FakeSession:
            def __init__(self):
                self.last = {'status': 'running'}
                self.com1_out = b''
                self.commands = []
            def send(self, command):
                self.commands.append(command)
                if command == f'run {term.BURST}':
                    self.last = {'status': 'exit'}
                return self.last
        session = FakeSession()
        port = MagicMock(); port.read.return_value = b''
        with patch.object(term, 'HeadlessGraphics') as graphics, patch.object(term.time, 'monotonic', return_value=0):
            graphics.return_value.stop_requested = False
            self.assertEqual(term.run_headless(session, port, 0, is_shell=False)['status'], 'exit')
            self.assertIn(f'run {term.SLICE}', session.commands)
            self.assertIn(f'run {term.BURST}', session.commands)
            graphics.return_value.update.assert_called_once()
            graphics.return_value.close.assert_called_once()

    def test_tk_closes_when_dos_session_exits(self):
        try:
            import tkinter as tk
        except ImportError as exc:
            self.skipTest(f'tkinter unavailable: {exc}')
        import time
        try:
            root = tk.Tk()
        except tk.TclError as exc:
            self.skipTest(f'Tk display unavailable: {exc}')
        root.withdraw()
        with tempfile.TemporaryDirectory() as directory:
            program = Path(directory) / 'exit.com'
            program.write_bytes(bytes.fromhex('b8004ccd21'))
            session = term.make_app(root, term.find_engine(), str(program), stop_log='')
            deadline = time.monotonic() + 5
            while session.process.poll() is None and time.monotonic() < deadline:
                root.update()
                time.sleep(0.01)
            self.assertIsNotNone(session.process.poll(), 'DOS exit did not close the session')
            with self.assertRaises(tk.TclError):
                root.winfo_exists()

    def test_cargo_engine_reads_config_and_runs_configured_program(self):
        engine = term.find_engine()
        with tempfile.TemporaryDirectory() as d:
            base = Path(d)
            (base / 'tail.com').write_bytes(TAIL)
            (base / 'cdrive').mkdir()
            config = base / 'pildos.conf'
            config.write_text('program=tail.com\nargs=/C configured\nC=cdrive\ncountry=358\n', encoding='utf-8')
            # An explicit config takes precedence over a stale environment setting.
            with patch.dict(term.os.environ, {'PILDOS_CONFIG': str(base / 'missing.conf')}):
                program, args = term.configured_startup(engine, config)
                self.assertEqual(Path(program), base / 'tail.com')
                self.assertEqual(args, '/C configured')
                session = term.Session(engine, program, args=args, config=config)
                try:
                    reply = session.send('run 1000')
                    self.assertEqual(reply['status'], 'exit')
                    self.assertEqual(term.screen_rows(reply)[0][0].rstrip(), ' /C configured')
                finally:
                    session.close()

    def test_headless_routes_console_before_execution_and_forwards_serial(self):
        class FakeSession:
            def __init__(self):
                self.last = {'status': 'running'}
                self.com1_out = b''
                self.commands = []
            def send(self, command):
                self.commands.append(command)
                if command.startswith('run '):
                    self.com1_out += b'Hello\r\n'
                    self.last = {'status': 'exit'}
                return self.last
        session = FakeSession();port = MagicMock();port.read.return_value = b'X\r'
        reply = term.run_headless(session, port, 7200, 358)
        self.assertEqual(reply['status'], 'exit')
        self.assertEqual(session.commands[:4], ['clock 7200', 'country 358', 'console com1', 'com1 580d'])
        port.write.assert_called_once_with(b'Hello\r\n')

    def test_headless_defaults_com1_and_ctty_args(self):
        with patch.object(term, 'open_com1') as mock_open, \
             patch.object(term, 'Session') as mock_session, \
             patch.object(term, 'run_headless', return_value={'status': 'exit'}):
            term.main(['--headless', '--shell', 'images/FD14LITE.img::COMMAND.COM'])
            mock_open.assert_called_once_with('console')
            mock_session.assert_called_once()
            self.assertEqual(mock_session.call_args[0][3], '/K CTTY COM1')
            self.assertTrue(term.run_headless.call_args[1]['is_shell'])

    def test_headless_shell_only_from_the_flag_not_the_name(self):
        # The program's name never decides: without --shell even COMMAND.COM
        # gets no command tail and ends on Esc.
        with patch.object(term, 'open_com1'), \
             patch.object(term, 'Session') as mock_session, \
             patch.object(term, 'run_headless', return_value={'status': 'exit'}) as mock_run:
            term.main(['--headless', 'images/FD14LITE.img::COMMAND.COM'])
            self.assertIsNone(mock_session.call_args[0][3])
            self.assertFalse(mock_run.call_args[1]['is_shell'])

    def test_headless_fallback_injects_ctty_when_waiting_input_without_traffic(self):
        class FakeWaitingSession:
            def __init__(self):
                self.last = {'status': 'running'}
                self.com1_out = b''
                self.commands = []
                self.runs = 0
            def send(self, command):
                self.commands.append(command)
                if command.startswith('run '):
                    self.runs += 1
                    if self.runs == 1:
                        self.last = {'status': 'waiting_input'}
                    else:
                        self.com1_out += b'C:\\>'
                        self.last = {'status': 'exit'}
                return self.last
        session = FakeWaitingSession();port = MagicMock();port.read.return_value = b''
        reply = term.run_headless(session, port, 0)
        self.assertEqual(reply['status'], 'exit')
        # Verify key commands for 'ctty com1\r' were sent
        keys_sent = [cmd for cmd in session.commands if cmd.startswith('key ')]
        self.assertEqual(len(keys_sent), 10) # 'c','t','t','y',' ','c','o','m','1','\r'

    def test_multiple_startup_drives_are_sent_before_running(self):
        self.assertEqual(term.drive_spec('a=disk one.img'), ('A', 'disk one.img'))
        self.assertEqual(term.drive_spec('B:=disk2.img'), ('B', 'disk2.img'))
        with self.assertRaises(term.argparse.ArgumentTypeError):
            term.drive_spec('Z=disk.img')
        replies = ''.join(json.dumps({'protocol': 1, 'seq': seq, 'ok': True}) + '\n' for seq in range(3))
        process = SimpleNamespace(stdin=io.StringIO(), stdout=io.StringIO(replies), stderr=io.StringIO())
        with patch.object(term.subprocess, 'Popen', return_value=process):
            term.Session('engine', 'program', drives=[('A', 'disk one.img'), ('B', 'disk2.img')])
        self.assertEqual(process.stdin.getvalue(), 'mount A disk one.img\nmount B disk2.img\n')

    def test_imported_drive_uses_label_and_refuses_existing_folder(self):
        import os
        previous = os.getcwd()
        with tempfile.TemporaryDirectory() as tmp:
            try:
                os.chdir(tmp)
                def run(cmd, **kwargs):
                    if cmd[1] == '--dos-drive-name':
                        return SimpleNamespace(returncode=0, stdout='MY_DISK-12345678\n', stderr='')
                    Path(cmd[3]).mkdir()
                    (Path(cmd[3]) / 'CONFIG.SYS').write_text('original')
                    return SimpleNamespace(returncode=0, stdout='Copied', stderr='')
                with patch.object(term.subprocess, 'run', side_effect=run):
                    drives = term.prepare_image_drives('engine', [('A', 'disk.img')], [('B', 'other.img')])
                    folder = Path('drives') / 'A' / 'MY_DISK-12345678'
                    self.assertEqual(drives, [('B', 'other.img'), ('A', str(folder))])
                    (folder / 'CONFIG.SYS').write_text('saved')
                    with self.assertRaisesRegex(ValueError, 'reuse it with --drive'):
                        term.prepare_image_drives('engine', [('A', 'disk.img')], [])
                    self.assertEqual((folder / 'CONFIG.SYS').read_text(), 'saved')
                    with self.assertRaisesRegex(ValueError, 'specified more than once'):
                        term.prepare_image_drives('engine', [('A', 'disk.img')], [('A', 'other.img')])
            finally:
                os.chdir(previous)

    def test_speaker_click_is_short_quiet_and_configurable(self):
        import struct
        import wave
        for level in (0, 5, 100):
            with wave.open(io.BytesIO(term.speaker_click(level)), 'rb') as wav:
                self.assertEqual((wav.getnchannels(), wav.getsampwidth(), wav.getframerate()), (1, 2, 22050))
                self.assertLessEqual(wav.getnframes() / wav.getframerate(), .011)
                raw = wav.readframes(wav.getnframes())
                samples = struct.unpack('<' + 'h' * (len(raw) // 2), raw)
                self.assertLessEqual(max(map(abs, samples)), 32767 * level / 100)
                self.assertEqual((samples[0], samples[-1]), (0, 0))
        for value in ('-1', '101', 'loud'):
            with self.assertRaises(term.argparse.ArgumentTypeError):
                term.speaker_volume(value)
        self.assertEqual(term.speaker_volume('0'), 0)
        self.assertEqual(term.speaker_volume('100'), 100)

    def test_speaker_coalesces_and_rate_limits_repeated_edges(self):
        player = MagicMock()
        now = [0.0]
        speaker = term.QuietSpeaker(5, initial=3, player=player, clock=lambda: now[0])
        speaker.observe({'beeps': 3})
        self.assertEqual(player.play.call_count, 0)
        speaker.observe({'beeps': 4})
        self.assertEqual(player.play.call_count, 1)
        now[0] = .1
        speaker.observe({'beeps': 20})
        self.assertEqual(player.play.call_count, 1)
        now[0] = .21
        speaker.observe({'beeps': 21})
        self.assertEqual(player.play.call_count, 2)
        speaker.observe({'beeps': 21})
        speaker.observe({'beeps': 1})  # restarting the count is not a beep
        self.assertEqual(player.play.call_count, 2)
        muted = term.QuietSpeaker(0, player=player, clock=lambda: 1)
        muted.observe({'beeps': 4})
        self.assertEqual(player.play.call_count, 2)

    def test_guest_speaker_pulses_coalesce_to_a_short_host_click(self):
        # Program PIT channel 2 and toggle port 61h 100 times, then exit.
        guest = bytes.fromhex('B0 B6 E6 43 B0 A9 E6 42 B0 04 E6 42 B9 64 00 '
                              'B0 03 E6 61 B0 00 E6 61 E2 F6 B8 00 4C CD 21')
        with tempfile.TemporaryDirectory() as directory:
            program = Path(directory) / 'SPEAK.COM'
            program.write_bytes(guest)
            session = term.Session(term.find_engine(), str(program), stop_log='')
            try:
                reply = session.send('run 1000')
                self.assertEqual((reply['status'], reply['beeps']), ('exit', 100))
                player = MagicMock()
                speaker = term.QuietSpeaker(5, player=player, clock=lambda: 1)
                speaker.observe(reply)
                speaker.observe(reply)
                self.assertEqual(player.play.call_count, 1)
            finally:
                session.close()

    def test_speaker_playback_runs_off_gui_thread_and_stays_bounded(self):
        import threading
        started, release = threading.Event(), threading.Event()
        player = term.AsyncSpeakerPlayer()
        def blocked(data):
            started.set()
            release.wait(2)
        player.output = MagicMock(side_effect=blocked)
        try:
            player.play(b'one')
            self.assertTrue(started.wait(1))
            player.play(b'two')
            self.assertEqual(player.output.call_count, 1)
        finally:
            release.set()
            player.worker.join(2)

    def test_ansi_option_reaches_the_session_from_window_constructor(self):
        try:
            import tkinter.font  # noqa: F401  (patched below; the module must exist)
        except ImportError as exc:
            self.skipTest(f'tkinter unavailable: {exc}')
        reply = {'screen': '2007' * 80 * 25, 'columns': 80, 'cursor': [0, 0],
                 'status': 'running', 'detail': '', 'steps': 0, 'cs': 0, 'eip': 0}
        with patch('tkinter.Text'), patch('tkinter.Label'), patch('tkinter.Menu'), \
             patch('tkinter.font.Font'), patch.object(term, 'Session') as session_class:
            session = session_class.return_value
            session.last = reply
            root = MagicMock()
            term.make_app(root, 'engine', 'program', utc_offset=0, ansi=True)
            self.assertTrue(session_class.call_args.kwargs['ansi'])
            session.send.reset_mock()
            term.make_app(root, 'engine', 'program', utc_offset=0)
            self.assertFalse(session_class.call_args.kwargs['ansi'])

    def test_session_ansi_option_installs_driver_before_running_guest(self):
        for enabled in (False, True):
            with self.subTest(ansi=enabled):
                replies = ''.join(json.dumps({'protocol': 1, 'seq': seq, 'ok': True}) + '\n'
                                  for seq in range(2))
                process = SimpleNamespace(stdin=io.StringIO(), stdout=io.StringIO(replies),
                                          stderr=io.StringIO())
                with patch.object(term.subprocess, 'Popen', return_value=process):
                    session = term.Session('engine', 'program', ansi=enabled)
                self.assertEqual(process.stdin.getvalue(), 'ansi on\n' if enabled else '')
                self.assertEqual(session.last['seq'], int(enabled))

    def test_cga_frame_bank_layout_palette_and_monochrome(self):
        frame = bytearray(0x4000)
        frame[0] = 0b00011011
        frame[0x2000] = 0b11000000
        rgb = term.cga_rgb(frame.hex(), 4, 0x30)
        self.assertEqual(rgb[:12], bytes.fromhex('00000055ffffff55ffffffff'))
        self.assertEqual(rgb[320*3:320*3+3], bytes.fromhex('ffffff'))
        rgb = term.cga_rgb(frame.hex(), 6, 0x0f)
        self.assertEqual(rgb[:12], bytes.fromhex('000000000000000000ffffff'))
        self.assertEqual(rgb[640*3:640*3+3], bytes.fromhex('ffffff'))
        with self.assertRaises(ValueError):
            term.cga_rgb('00', 4, 0x30)

    def test_planar_frame_combines_four_planes_through_sixteen_colours(self):
        size = 80 * 480
        planes = bytearray(4 * size)
        planes[0] = 0x80               # pixel 0: plane 0 -> colour 1
        planes[size] = 0x40            # pixel 1: plane 1 -> colour 2
        planes[3 * size + 1] = 0x01    # pixel 15: plane 3 -> colour 8
        colors = ''.join(f'{i:02x}{i:02x}{i:02x}' for i in range(16))
        rgb = term.planar_rgb(planes.hex(), 640, 480, colors)
        self.assertEqual(len(rgb), 640 * 480 * 3)
        self.assertEqual(rgb[:9], bytes([1, 1, 1, 2, 2, 2, 0, 0, 0]))
        self.assertEqual(rgb[15 * 3:16 * 3], bytes([8, 8, 8]))
        with self.assertRaises(ValueError):
            term.planar_rgb(planes.hex(), 640, 350, colors)

    def test_cga_mode_five_is_cyan_red_white_whatever_the_palette_bit(self):
        frame = bytearray(0x4000)
        frame[0] = 0b00011011
        for color in (0x30, 0x10):
            self.assertEqual(term.cga_rgb(frame.hex(), 5, color)[:12], bytes.fromhex('00000055ffffff5555ffffff'))
        self.assertEqual(term.cga_rgb(frame.hex(), 5, 0x00)[:12], bytes.fromhex('00000000aaaaaa0000aaaaaa'))

    def test_graphics_palette_maps_indices_and_rejects_bad_data(self):
        palette = bytes([i for i in range(256) for _ in range(3)])
        pixels = bytes([0, 4, 255]) + bytes(63997)
        self.assertEqual(term.graphics_rgb(pixels.hex(), palette.hex())[:9], bytes([0,0,0,4,4,4,255,255,255]))
        with self.assertRaises(ValueError):
            term.graphics_rgb('00', palette.hex())

    def test_graphics_modes_display_at_four_by_three_with_nearest_rows(self):
        # Distinct scanline colours make duplicated rows and both edges visible.
        for width in (320, 640):
            with self.subTest(width=width):
                rgb = b''.join(bytes((y, 255 - y, y)) * width for y in range(200))
                tk = MagicMock()
                result = term.graphics_photo(tk, rgb, width)
                data = tk.PhotoImage.call_args.kwargs['data']
                header, raster = data.split(b'\n', 1)
                self.assertEqual(header, f'P6 {width} 480 255'.encode())
                row_bytes = width * 3
                self.assertEqual(len(raster), row_bytes * 480)
                for y in range(480):
                    source_y = y * 200 // 480
                    self.assertEqual(raster[y * row_bytes:(y + 1) * row_bytes],
                                     rgb[source_y * row_bytes:(source_y + 1) * row_bytes])
                if width == 320:
                    tk.PhotoImage.return_value.zoom.assert_called_once_with(2, 1)
                    self.assertIs(result, tk.PhotoImage.return_value.zoom.return_value)
                else:
                    tk.PhotoImage.return_value.zoom.assert_not_called()
                    self.assertIs(result, tk.PhotoImage.return_value)
        with self.assertRaises(ValueError):
            term.graphics_photo(MagicMock(), b'', 320)
        with self.assertRaises(ValueError):
            term.graphics_photo(MagicMock(), b'', 800)

    def test_tk_graphics_photo_dimensions_and_colours(self):
        try:
            import tkinter as tk
        except ImportError as exc:
            self.skipTest(f'tkinter unavailable: {exc}')
        try:
            root = tk.Tk()
        except tk.TclError as exc:
            self.skipTest(f'Tk display unavailable: {exc}')
        root.withdraw()
        try:
            for width in (320, 640):
                with self.subTest(width=width):
                    rgb = b''.join(bytes((y, 255 - y, 0)) * width for y in range(200))
                    image = term.graphics_photo(tk, rgb, width)
                    self.assertEqual((image.width(), image.height()), (640, 480))
                    for y in (0, 2, 3, 239, 479):
                        source_y = y * 200 // 480
                        self.assertEqual(image.get(639, y), (source_y, 255 - source_y, 0))
        finally:
            root.destroy()

    def test_tk_switches_between_text_and_each_graphics_mode(self):
        try:
            import tkinter as tk
        except ImportError as exc:
            self.skipTest(f'tkinter unavailable: {exc}')
        try:
            root = tk.Tk()
        except tk.TclError as exc:
            self.skipTest(f'Tk display unavailable: {exc}')
        root.withdraw()
        text_reply = {'screen': '2007' * 80 * 25, 'columns': 80, 'cursor': [0, 0],
                      'status': 'stopped', 'detail': '', 'steps': 0, 'cs': 0, 'eip': 0}
        base = {'status': 'stopped', 'detail': '', 'steps': 0}
        cga = '00' * 0x4000
        replies = [dict(base, graphics_cga=cga, cga_mode=4, cga_color=0),
                   dict(base, graphics_cga=cga, cga_mode=6, cga_color=15),
                   dict(base, graphics13='00' * 64000, palette='00' * 768),
                   text_reply]
        try:
            with patch.object(term, 'Session') as session_class, patch.object(root, 'after') as after:
                session = session_class.return_value
                session.last = text_reply
                term.make_app(root, 'engine', 'program', utc_offset=0, speaker_level=0)
                tick = after.call_args.args[1]
                text_widget = next(widget for widget in root.winfo_children() if isinstance(widget, tk.Text))
                graphics = None
                for index, reply in enumerate(replies):
                    session.last = reply
                    tick()
                    if index < 3:
                        graphics = next(widget for widget in root.winfo_children()
                                        if isinstance(widget, tk.Label) and hasattr(widget, 'image'))
                        self.assertEqual((graphics.image.width(), graphics.image.height()), (640, 480))
                        self.assertEqual(graphics.winfo_manager(), 'pack')
                        self.assertEqual(text_widget.winfo_manager(), '')
                    else:
                        self.assertEqual(graphics.winfo_manager(), '')
                        self.assertEqual(text_widget.winfo_manager(), 'pack')
        finally:
            root.destroy()

    def test_session_reads_graphics_with_large_escaped_output(self):
        reply = {'protocol': 1, 'seq': 0, 'graphics13': '00' * 64000,
                 'output': '\x00' * 16384, 'com1': ''}
        encoded = json.dumps(reply) + '\n'
        self.assertGreater(len(encoded), 200000)
        session = term.Session.__new__(term.Session)
        session.process = SimpleNamespace(stdout=io.StringIO(encoded), stderr=io.StringIO())
        session.seq = 0
        session.com1_out = b''
        self.assertEqual(session._read(), reply)

    def test_keys_map_to_bios_ascii_and_scan_codes(self):
        self.assertEqual(term.key_code('a', 'a'), (0x61, 0x1e))
        self.assertEqual(term.key_code('A', 'A'), (0x41, 0x1e))
        self.assertEqual(term.key_code('exclam', '!'), (0x21, 0x02))
        self.assertEqual(term.key_code('Return', '\r'), (13, 0x1c))
        self.assertEqual(term.key_code('Up', ''), (0, 0x48))
        self.assertEqual(term.key_code('F10', ''), (0, 0x44))
        self.assertEqual(term.key_code('F9', ''), (0, 0x43))  # Turbo: Compile
        self.assertEqual(term.key_code('F9', '', control=True), (0, 0x66))  # Turbo: Run
        self.assertEqual(term.key_code('F9', '', shift=True), (0, 0x5c))
        self.assertEqual(term.key_code('F9', '', alt=True), (0, 0x70))
        for n in range(1, 11):
            with self.subTest(function_key=n):
                self.assertEqual(term.key_code(f'F{n}', '', control=True), (0, 0x5d + n))
                self.assertEqual(term.key_code(f'F{n}', '', shift=True), (0, 0x53 + n))
        self.assertEqual(term.key_code('c', '\x03', True), (3, 0x2e))
        self.assertIsNone(term.key_code('Shift_L', ''))
        self.assertEqual(term.key_code('adiaeresis', 'ä'), (0x84, 0x28))
        self.assertEqual(term.key_code('Odiaeresis', 'Ö'), (0x99, 0x27))
        self.assertEqual(term.key_code('aring', 'å'), (0x86, 0x1a))
        self.assertIsNone(term.key_code('EuroSign', '€'))
        self.assertEqual(term.key_code('section', '§'), (0x15, 0x29))
        self.assertEqual(term.key_code('eacute', 'é'), (0x82, 0))
        self.assertEqual(term.key_code('f', 'f', alt=True), (0, 0x21))
        self.assertEqual(term.key_code('F', 'F', alt=True), (0, 0x21))
        self.assertEqual(term.key_code('1', '1', alt=True), (0, 0x78))
        self.assertEqual(term.key_code('F10', '', alt=True), (0, 0x71))
        self.assertEqual(term.key_code('Return', '\r', alt=True), (13, 0x1c))
        # AltGr on a Finnish layout: the characters, not Alt codes.
        self.assertEqual(term.key_code('backslash', '\\', True, True), (0x5c, 0x2b))
        self.assertEqual(term.key_code('at', '@', True, True), (0x40, 0x03))
        self.assertEqual(term.key_code('braceleft', '{', True, True), (0x7b, 0x1a))
        # Windows may give the Ctrl form of an AltGr character, or none.
        self.assertEqual(term.key_code('backslash', '\x1c', True, False, True), (0x5c, 0x2b))
        self.assertEqual(term.key_code('braceleft', '\x1b', True, True, True), (0x7b, 0x1a))
        self.assertEqual(term.key_code('at', '', True, False, True), (0x40, 0x03))
        self.assertEqual(term.key_code('mu', '', True, False, True), (0xe6, 0))
        # Ctrl+\ alone stays FS, and Ctrl+Alt+letter is still an Alt code.
        self.assertEqual(term.key_code('backslash', '\x1c', True), (0x1c, 0))
        self.assertEqual(term.key_code('e', '\x05', True, True, True), (0, 0x12))

    def test_windows_altgr_with_cleared_state_and_base_keysym(self):
        # Actual Windows Finnish keylog: held Ctrl_L/Alt_R, state 0,
        # keysyms 7/8/9/0 but correctly translated characters {/[ /]/}.
        held = {'Control_L', 'Alt_R'}
        with patch.object(term, 'ALTGR_IS_CTRL_ALT', True):
            for symbol, char, expected in [('7', '{', (123, 26)), ('8', '[', (91, 26)),
                                           ('9', ']', (93, 27)), ('0', '}', (125, 27))]:
                for state in (0, 0x2000c):
                    with self.subTest(symbol=symbol, state=state):
                        self.assertEqual(term.event_key_code(symbol, char, state, held), expected)
            # AltGr+< on a Finnish layout: keysym less, character |.
            for state in (0, 0x2000c):
                self.assertEqual(term.event_key_code('less', '|', state, held), (0x7c, 0x2b))
            self.assertEqual(term.event_key_code('less', '<', 0, set()), (0x3c, 0x33))
            # Unsupported euro must not become Alt+E (BBC BASIC's ELSE macro).
            self.assertIsNone(term.event_key_code('e', '€', 0, held))
            self.assertEqual(term.event_key_code('braceleft', '\x1b', 0, held), (123, 26))
            self.assertEqual(term.event_key_code('e', 'e', 0x8, {'Alt_L'}), (0, 18))
            self.assertEqual(term.event_key_code('7', '', 0x8, {'Alt_L'}), (0, 126))
            self.assertEqual(term.event_key_code('c', '\x03', 0x4, {'Control_L'}), (3, 46))
            self.assertEqual(term.event_key_code('F9', '', 0x4, {'Control_L'}), (0, 0x66))
            self.assertEqual(term.event_key_code('F9', '', 0, {'Control_L'}), (0, 0x66))
            self.assertEqual(term.event_key_code('F9', '', 0x1, {'Shift_L'}), (0, 0x5c))
            self.assertEqual(term.event_key_code('F9', '', 0x8, {'Alt_L'}), (0, 0x70))
            self.assertEqual(term.event_key_code('F9', '', 0x0, set()), (0, 0x43))
            self.assertEqual(term.event_key_code('7', '7', 0, set()), (55, 8))

    def test_utc_offsets_parse_as_hours_or_hours_and_minutes(self):
        self.assertEqual([term.parse_offset(t) for t in ('2', '-5', '5.5', '+02:00', '-03:30', '0')],
                         [7200, -18000, 19800, 7200, -12600, 0])
        with self.assertRaises(ValueError):
            term.parse_offset('15')

    def test_pasted_text_becomes_keystrokes(self):
        self.assertEqual(term.paste_keys('Dir\r\nä\t€x'),
                         [(0x44, 0x20), (0x69, 0x17), (0x72, 0x13), (13, 0x1c), (0x84, 0x28), (9, 0x0f), (0x78, 0x2d)])

    def test_screen_rows_decode_cp437_characters_and_attributes(self):
        cells = bytearray(b' \x07' * 80 * 25)
        cells[0:6] = b'\xc9\x1fA\x1fB\x4e'
        rows = term.screen_rows({'screen': cells.hex(), 'columns': 80})
        self.assertEqual(len(rows), 25)
        self.assertEqual(rows[0][0][:3], '╔AB')
        self.assertEqual(rows[0][1][:4], [0x1f, 0x1f, 0x4e, 0x07])

    def test_all_256_characters_have_their_pc_glyph(self):
        cells = bytearray(b' \x07' * 80 * 25)
        cells[0:512:2] = bytes(range(256))
        text = ''.join(row[0] for row in term.screen_rows({'screen': cells.hex(), 'columns': 80}))[:256]
        self.assertEqual(text[:4] + text[0x0f] + text[0x1f] + text[0x7f], ' ☺☻♥☼▼⌂')
        self.assertEqual(text[0x80] + text[0xb0] + text[0xdb] + text[0xe1] + text[0xfe], 'Ç░█ß■')
        self.assertEqual(len(set(text[1:])), 255, 'no two codes share a glyph except 00h/20h')

    def test_session_runs_waits_for_keys_and_exits(self):
        engine = term.find_engine()
        with tempfile.TemporaryDirectory() as d:
            program = Path(d) / 'ask.com'
            program.write_bytes(ASK)
            session = term.Session(engine, program)
            try:
                reply = session.send('run 1000')
                self.assertEqual(reply['status'], 'waiting_input')
                self.assertEqual(term.screen_rows(reply)[0][0].rstrip(), 'Name?')
                self.assertTrue(session.send(f'clock {term.local_offset()}')['ok'])
                self.assertFalse(session.send('clock 99999')['ok'])
                self.assertTrue(session.send('shift 8')['ok'])
                self.assertFalse(session.send('shift 16')['ok'])
                self.assertTrue(session.send('shift 0')['ok'])
                self.assertTrue(session.send('country 358')['ok'])
                self.assertFalse(session.send('country 49')['ok'])
                self.assertTrue(session.send('ansi on')['ok'])
                self.assertFalse(session.send('ansi maybe')['ok'])
                for ascii_code, scan in [term.key_code('h', 'h'), term.key_code('i', 'i')]:
                    self.assertTrue(session.send(f'key {ascii_code} {scan}')['ok'])
                reply = session.send('run 1000')
                self.assertEqual(term.screen_rows(reply)[0][0].rstrip(), 'Name? hi')
                self.assertEqual(reply['cursor'], [0, 8])
                session.send('key 13 28')
                reply = session.send('run 1000')
                self.assertEqual((reply['status'], reply['detail']), ('exit', 'exit code 0'))
                self.assertFalse(session.send('run 0')['ok'])
            finally:
                session.close()

    def test_ctrl_break_keys_and_ctrl_c_ends_a_program_without_a_handler(self):
        self.assertTrue(term.is_ctrl_break('Cancel', True))
        self.assertTrue(term.is_ctrl_break('Pause', True))
        self.assertFalse(term.is_ctrl_break('Pause', False))
        self.assertFalse(term.is_ctrl_break('c', True))
        engine = term.find_engine()
        with tempfile.TemporaryDirectory() as d:
            program = Path(d) / 'ask.com'
            program.write_bytes(ASK)
            for key in (f'key {3} {0x2e}', 'break'):
                with self.subTest(key=key):
                    session = term.Session(engine, program)
                    try:
                        self.assertEqual(session.send('run 1000')['status'], 'waiting_input')
                        self.assertTrue(session.send(key)['ok'])
                        reply = session.send('run 1000')
                        self.assertEqual((reply['status'], reply['detail']), ('exit', 'exit code 0'))
                        self.assertEqual(term.screen_rows(reply)[0][0].rstrip(), 'Name? ^C')
                        self.assertEqual(reply['cursor'], [1, 0])
                    finally:
                        session.close()

    def test_refused_keys_wait_only_for_room(self):
        self.assertTrue(term.is_retryable('BIOS keyboard buffer full'))
        self.assertTrue(term.is_retryable('keyboard scan codes still waiting for the program'))
        self.assertFalse(term.is_retryable("the program hooked INT 1Bh; calling it for Ctrl+Break is not modelled"))

    def test_args_reach_the_program_command_line(self):
        engine = term.find_engine()
        with tempfile.TemporaryDirectory() as d:
            program = Path(d) / 'tail.com'
            program.write_bytes(TAIL)
            session = term.Session(engine, program, args='/K D:\\START.BAT')
            try:
                reply = session.send('run 1000')
                self.assertEqual((reply['status'], term.screen_rows(reply)[0][0].rstrip()), ('exit', ' /K D:\\START.BAT'))
            finally:
                session.close()

    def test_com1_bytes_are_code_page_437_with_cr_for_enter(self):
        self.assertEqual(term.to_cp437('dir\nä€'), b'dir\r\x84')
        self.assertEqual(term.from_cp437(b'\x1b[1m\xc9\xcd'), '\x1b[1m╔═')

    def test_mount_and_unmount_commands_validate_drives_and_paths(self):
        self.assertEqual(term.mount_command('a:', '/media/usb/disk1.img'), 'mount A /media/usb/disk1.img')
        self.assertEqual(term.mount_command('D', 'C:\\My Floppies'), 'mount D C:\\My Floppies')
        self.assertEqual(term.unmount_command('b:'), 'unmount B')
        for bad in ('Z', 'F', ''):
            with self.assertRaises(ValueError):
                term.mount_command(bad, '/tmp')
            with self.assertRaises(ValueError):
                term.unmount_command(bad)
        with self.assertRaises(ValueError):
            term.mount_command('A', '   ')

    def test_console_com1_drops_vt_key_sequences_but_keeps_esc(self):
        self.assertEqual(term.strip_vt_keys(b'dir\x1b[A\x1b[1;5C\x1bOP\x1b[2~x'), b'dirx')
        self.assertEqual(term.strip_vt_keys(b'\x1b'), b'\x1b')
        self.assertEqual(term.strip_vt_keys(b'ab\x1bc\r'), b'ab\x1bc\r')

    def test_tcp_port_passes_bytes_both_ways(self):
        import socket
        import time
        port = term.TcpPort(0)
        try:
            client = socket.create_connection(port.server.getsockname())
            client.settimeout(5)
            for _ in range(500):
                if port._accept():
                    break
                time.sleep(0.01)
            port.write(b'C:\\>')
            self.assertEqual(client.recv(16), b'C:\\>')
            client.sendall(b'VER\r\n')
            data = b''
            for _ in range(500):
                data += port.read()
                if data:
                    break
                time.sleep(0.01)
            self.assertEqual(data, b'VER\r')
            client.close()
        finally:
            port.server.close()


if __name__ == '__main__':
    unittest.main()



class KeyEdgesTests(unittest.TestCase):
    def test_guest_press_remains_held_until_real_release(self):
        events = term.KeyEvents(True)
        self.assertEqual(events.press('Escape', (27, 1)), 'keydown 27 1')
        self.assertEqual(events.pressed, {'Escape': (27, 1)})
        self.assertEqual(events.release('Escape'), 'keyup 27 1')
        self.assertIsNone(events.release('Escape'))
        self.assertEqual(events.pressed, {})

    def test_console_and_older_engines_keep_the_legacy_key_command(self):
        for events, raw in [(term.KeyEvents(False), True), (term.KeyEvents(True), False)]:
            self.assertEqual(events.press('a', (97, 30), raw), 'key 97 30')
            self.assertIsNone(events.release('a'))

    def test_focus_loss_releases_all_guest_keys_with_original_codes(self):
        events = term.KeyEvents(True)
        events.press('a', (65, 30))
        events.press('Up', (0, 72))
        self.assertEqual(events.release_all(), ['keyup 65 30', 'keyup 0 72'])
        self.assertEqual(events.release_all(), [])

    def test_headless_tui_renders_alt_screen_diffs_and_restores_on_leave(self):
        out = bytearray()
        port = type('Port', (), {'write': lambda self, d: out.extend(d)})()
        tui = term.HeadlessTui()
        blank = '2007' * (80 * 25)
        first = '411e' + blank[4:]
        reply = {'columns': 80, 'cursor': [0, 1], 'screen': first}
        tui.enter(port, reply)
        self.assertTrue(tui.active)
        self.assertIn(b'\x1b[?1049h\x1b[2J', out)
        self.assertIn(b'\x1b[1;1H\x1b[93;44mA', out)
        self.assertIn(b'\x1b[1;2H\x1b[?25h', out)
        out.clear()
        tui.update(port, reply)
        self.assertEqual(bytes(out), b'')
        tui.leave(port)
        self.assertFalse(tui.active)
        self.assertEqual(bytes(out), b'\x1b[0m\x1b[?25h\x1b[?1049l')

    def test_configured_frontend_reads_terminal_flags_from_config(self):
        import tempfile
        with tempfile.TemporaryDirectory() as tmp:
            conf = Path(tmp) / 'pildos.conf'
            conf.write_text('#@program=images/FD14LITE.img::COMMAND.COM\nheadless=true\nrust_graphics=true\nmouse=true\nfpu=80387\nshell=true\nengine=target/release/pil.exe\n', encoding='utf-8')
            opts = term.configured_frontend(str(conf))
            self.assertEqual(opts['headless'], True)
            self.assertEqual(opts['rust_graphics'], True)
            self.assertEqual(opts['mouse'], True)
            self.assertEqual(opts['fpu'], '80387')
            self.assertEqual(opts['shell'], True)
            self.assertEqual(Path(opts['engine']), Path(tmp) / 'target/release/pil.exe')

    def test_console_port_read_forwards_extended_key_tuples_to_com1(self):
        import queue
        port = term.ConsolePort.__new__(term.ConsolePort)
        port.input = queue.Queue()
        port.input.put(b'ab')
        port.input.put((0, 75))
        port.input.put(b'c')
        self.assertEqual(port.read(), b'ab\x00Kc')

    def test_raw_key_sequence_wraps_alt_ctrl_and_shift_combos(self):
        self.assertEqual(term.raw_key_sequence(0, 33), (56, 0, 33))    # Alt+F
        self.assertEqual(term.raw_key_sequence(0, 45), (56, 0, 45))    # Alt+X
        self.assertEqual(term.raw_key_sequence(0, 102), (29, 0, 67))   # Ctrl+F9
        self.assertEqual(term.raw_key_sequence(0, 112), (56, 0, 67))   # Alt+F9
        self.assertEqual(term.raw_key_sequence(0, 84), (42, 0, 59))    # Shift+F1
        self.assertEqual(term.raw_key_sequence(ord('a'), 30), (None, ord('a'), 30))
