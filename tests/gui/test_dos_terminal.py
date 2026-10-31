import importlib.util
from pathlib import Path
import tempfile
import unittest
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
        self.assertIsNone(term.key_code('Return', '\r', alt=True))

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


if __name__ == '__main__':
    unittest.main()
