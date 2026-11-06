import importlib.util
import io
import json
from types import SimpleNamespace
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
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

    def test_graphics_palette_maps_indices_and_rejects_bad_data(self):
        palette = bytes([i for i in range(256) for _ in range(3)])
        pixels = bytes([0, 4, 255]) + bytes(63997)
        self.assertEqual(term.graphics_rgb(pixels.hex(), palette.hex())[:9], bytes([0,0,0,4,4,4,255,255,255]))
        with self.assertRaises(ValueError):
            term.graphics_rgb('00', palette.hex())

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

