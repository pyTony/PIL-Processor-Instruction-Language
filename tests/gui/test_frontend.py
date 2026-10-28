import importlib.util
import json
from pathlib import Path
import tempfile
import threading
import time
import unittest
spec=importlib.util.spec_from_file_location('pil_frontend',Path(__file__).resolve().parents[2]/'tools/pil_debugger.py')
front=importlib.util.module_from_spec(spec);spec.loader.exec_module(front)

class FrontendTests(unittest.TestCase):
    def message(self):
        state={key:'' for key in ['version','demo','listing','pil','memory','stack','segments','cache']}
        state.update(registers=[0]*8,eip=256,flags=514,undefined_flags=0,memory_address=0,stack_address=0,ram_size=4096,breakpoints=[])
        return dict(protocol=1,seq=0,ok=True,quit=False,text='ä\\\n',status='ready',state=state,guest_output=[0,255])

    def test_binary_bytes_and_text_remain_separate(self):
        msg=self.message();decoded=front.decode_message(json.dumps(msg)+'\n',0)
        self.assertEqual(decoded['text'],msg['text']);self.assertEqual(decoded['guest_output'],[0,255])

    def test_rejects_out_of_order_invalid_byte_and_incomplete_snapshot(self):
        for change in ['sequence','byte','register','snapshot','status']:
            msg=self.message()
            if change=='sequence':msg['seq']=1
            elif change=='byte':msg['guest_output']=[256]
            elif change=='register':msg['state']['registers'][0]=True
            elif change=='snapshot':del msg['state']['listing']
            else:msg['status']='unknown'
            with self.assertRaises(front.ProtocolError):front.decode_message(json.dumps(msg)+'\n',0)
        with self.assertRaises(front.ProtocolError):front.decode_message(json.dumps(self.message()),0)

    def test_pause_and_wait_never_schedule_another_run(self):
        for status in front.STATUSES:
            self.assertIsNone(front.next_command(False,dict(ok=True,status=status)))
        for status in ['halt','waiting_rx','breakpoint','fault','decode','missing_instruction','error']:
            self.assertIsNone(front.next_command(True,dict(ok=True,status=status)))
        self.assertEqual(front.next_command(True,dict(ok=True,status='tx_blocked')),'serial tx')
        self.assertIsNone(front.next_command(True,dict(ok=False,status='budget')))

    def test_explicit_engine_path_with_spaces_is_an_argument_not_shell_code(self):
        with tempfile.TemporaryDirectory() as d:
            path=Path(d)/'engine file';path.write_bytes(b'')
            self.assertEqual(front.find_engine(str(path)),path.resolve())
        with self.assertRaises(FileNotFoundError):front.find_engine('/no-such-pil-backend')

    def test_command_history_navigation_and_bounds(self):
        history = front.CommandHistory(limit=3)
        self.assertIsNone(history.previous())
        self.assertIsNone(history.next())
        history.append('step')
        history.append('run 10')
        history.append('run 10')  # deduplicate consecutive identical commands
        self.assertEqual(len(history.entries), 2)
        self.assertEqual(history.previous(), 'run 10')
        self.assertEqual(history.previous(), 'step')
        self.assertEqual(history.previous(), 'step')  # clamp at start
        self.assertEqual(history.next(), 'run 10')
        self.assertEqual(history.next(), '')  # past end returns empty line
        history.append('step')
        history.append('break 0x100')
        history.append('halt')
        self.assertEqual(len(history.entries), 3)  # limit enforced
        self.assertNotIn('run 10', history.entries)

    def test_parse_hex_program_and_bounded_reader(self):
        self.assertEqual(front.parse_hex_program('b0 41 e6 e9 f4'), b'\xb0\x41\xe6\xe9\xf4')
        self.assertEqual(front.parse_hex_program('0xb0 0x41 ; comment\ne6 e9 # comment\nf4'), b'\xb0\x41\xe6\xe9\xf4')
        with self.assertRaises(ValueError):
            front.parse_hex_program('')
        with self.assertRaises(ValueError):
            front.parse_hex_program('xyz 12')
        with self.assertRaises(ValueError):
            front.parse_hex_program('100')  # byte value out of range (256)
        with self.assertRaises(ValueError):
            front.parse_hex_program('90 ' * 5, max_bytes=4)
        with tempfile.TemporaryDirectory() as d:
            ok_hex = Path(d) / 'ok.hex'
            ok_hex.write_text('b8 2a 00 00 00 f4\n', encoding='utf-8')
            self.assertEqual(front.read_bounded_hex_file(ok_hex), b'\xb8\x2a\x00\x00\x00\xf4')
            huge_text = Path(d) / 'huge.hex'
            huge_text.write_text('; ' + ('x' * 200) + '\n90\n', encoding='utf-8')
            with self.assertRaises(ValueError):
                front.read_bounded_hex_file(huge_text, max_text_bytes=64)

    def test_async_job_runner_does_not_block_caller_and_drains_on_poll(self):
        runner = front.AsyncJobRunner()
        gate = threading.Event()
        delivered = []

        def slow_work():
            gate.wait(timeout=2.0)
            return [('FDOS/BIN/MEM.EXE', 47551, 'EXE')]

        t0 = time.monotonic()
        accepted = runner.submit(slow_work, lambda res, err: delivered.append((res, err)))
        elapsed = time.monotonic() - t0
        self.assertTrue(accepted)
        self.assertLess(elapsed, 0.05)  # returns immediately on UI thread
        self.assertFalse(runner.submit(lambda: None, lambda r, e: None))  # single active job guard
        self.assertEqual(runner.drain(), 0)
        gate.set()
        deadline = time.monotonic() + 2.0
        while not delivered and time.monotonic() < deadline:
            runner.drain()
            time.sleep(0.005)
        self.assertEqual(delivered, [([('FDOS/BIN/MEM.EXE', 47551, 'EXE')], None)])

    def test_parse_container_listing(self):
        sample = (
            'CONTAINER fat16 41943040 FD14LITE.img\n'
            '      58504 COMMAND.COM\n'
            '      47551 FDOS/BIN/MEM.EXE\n'
            '      12345 FDOS/PACKAGES/BASE.ZIP\n'
            '        128 AUTOEXEC.BAT\n'
            'NOTE 4 files matched\n'
        )
        self.assertEqual(
            front.parse_container_listing(sample),
            [
                ('COMMAND.COM', 58504, 'COM'),
                ('FDOS/BIN/MEM.EXE', 47551, 'EXE'),
                ('FDOS/PACKAGES/BASE.ZIP', 12345, 'ZIP'),
                ('AUTOEXEC.BAT', 128, 'FILE'),
            ],
        )

    def test_memory_cell_formatting_and_highlight_spans(self):
        dump = (
            '00000500: 41 00 80 ff 78 56 34 12 00 00 00 00 00 00 00 00 |A...xV4.........|\n'
            '00000510: 00 00 00 00 00 00 00 00 00 00 00 00 00 fc 8c 1c |................|\n'
        )
        eff1 = {'address': 0x500, 'bytes': 1, 'write': True, 'stage': 'written', 'raw': [0x41]}
        text1 = front.format_memory_cell_value(eff1)
        self.assertIn('[0x00000500] (1 byte, written)', text1)
        self.assertIn('Hex: 0x41 · Dec: 65 · Bytes: [41] · Chars (1): |A| (\'A\')', text1)
        self.assertEqual(front.find_memory_highlight_spans(dump, eff1), [('1.10', '1.12'), ('1.59', '1.60')])

        eff2 = {'address': 0x502, 'bytes': 2, 'write': False, 'stage': 'read', 'raw': [0x80, 0xFF]}
        text2 = front.format_memory_cell_value(eff2)
        self.assertIn('[0x00000502..0x00000503] (2 bytes, read)', text2)
        self.assertIn('Hex: 0xFF80 · Dec: 65,408 (signed -128) · Bytes: [80 ff] · Chars (2): |..|', text2)
        self.assertEqual(front.find_memory_highlight_spans(dump, eff2), [('1.16', '1.21'), ('1.61', '1.63')])

        eff4 = {'address': 0x504, 'bytes': 4, 'write': True, 'stage': 'written', 'raw': [0x78, 0x56, 0x34, 0x12]}
        text4 = front.format_memory_cell_value(eff4)
        self.assertIn('[0x00000504..0x00000507] (4 bytes, written)', text4)
        self.assertIn("Hex: 0x12345678 · Dec: 305,419,896 · Bytes: [78 56 34 12] · Chars (4): |xV4.| ('x' 'V' '4' '\\x12')", text4)
        self.assertEqual(front.find_memory_highlight_spans(dump, eff4), [('1.22', '1.33'), ('1.63', '1.67')])

if __name__=='__main__':unittest.main()
