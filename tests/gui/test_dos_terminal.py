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
        self.assertIsNone(term.key_code('adiaeresis', 'ä'))

    def test_screen_rows_decode_cp437_characters_and_attributes(self):
        cells = bytearray(b' \x07' * 80 * 25)
        cells[0:6] = b'\xc9\x1fA\x1fB\x4e'
        rows = term.screen_rows({'screen': cells.hex(), 'columns': 80})
        self.assertEqual(len(rows), 25)
        self.assertEqual(rows[0][0][:3], '╔AB')
        self.assertEqual(rows[0][1][:4], [0x1f, 0x1f, 0x4e, 0x07])

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


if __name__ == '__main__':
    unittest.main()
