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

    def test_monitor_load_completion_keeps_newer_session_and_cleans_stale_file(self):
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / 'entry.bin'
            path.write_bytes(b'\x90')
            requested_session = object()
            session = requested_session
            loaded, errors = [], []
            runner = front.AsyncJobRunner()

            def completed(result, error):
                front.complete_monitor_load(
                    result, error, is_current=lambda: session is requested_session,
                    load=lambda *args: loaded.append(args), report_error=errors.append,
                )

            self.assertTrue(runner.submit(lambda: (str(path), 1), completed))
            session = object()  # user opened another program before UI delivery
            deadline = time.monotonic() + 2.0
            while path.exists() and time.monotonic() < deadline:
                runner.drain()
                time.sleep(0.005)
            self.assertEqual(loaded, [])
            self.assertEqual(errors, [])
            self.assertFalse(path.exists())

    def test_monitor_load_delivers_only_current_results_and_errors(self):
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / 'entry.bin'
            path.write_bytes(b'\x90')
            loaded, errors = [], []
            front.complete_monitor_load(
                (str(path), 1), None, is_current=lambda: True,
                load=lambda *args: loaded.append(args), report_error=errors.append,
            )
            self.assertEqual(loaded, [(str(path), 1)])
            self.assertTrue(path.exists())  # engine still needs to open it
            error = RuntimeError('read failed')
            for current in [False, True]:
                front.complete_monitor_load(
                    None, error, is_current=lambda: current,
                    load=lambda *args: self.fail('failed read must not start engine'), report_error=errors.append,
                )
            self.assertEqual(errors, [error])

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

    def _dosbox_module(self):
        dosbox_spec = importlib.util.spec_from_file_location(
            'verify_dosbox_cpu', Path(__file__).resolve().parents[2] / 'tools/verify_dosbox_cpu.py'
        )
        dosbox_mod = importlib.util.module_from_spec(dosbox_spec)
        dosbox_spec.loader.exec_module(dosbox_mod)
        return dosbox_mod

    def test_case_manifest_loader_and_unrelated_binary_isolation(self):
        dosbox_mod = self._dosbox_module()
        sample_cases = [
            {
                'name': 'xchg16_0',
                'width': 16,
                'eax': 0x13570012,
                'ecx': 0x246800fe,
                'edx': 0xc0deface,
                'code': '91',
                'expected_eax': 0x135700fe,
                'expected_ecx': 0x24680012,
                'expected_edx': 0xc0deface,
                'flags': 0x8d5,
                'flags_mask': 0xcd5,
                'incoming_flags': 0x202 | 0x8d5,
            }
        ]
        com_bytes = dosbox_mod.build(sample_cases)
        with tempfile.TemporaryDirectory() as d:
            manifest_path = Path(d) / 'cases.json'
            com_path = Path(d) / 'CPUTEST.COM'
            unrelated_path = Path(d) / 'unrelated.bin'
            small_path = Path(d) / 'small.bin'
            com_path.write_bytes(com_bytes)
            unrelated_path.write_bytes(b'\x90' * 4000)
            small_path.write_bytes(b'\x90\xf4')
            manifest_path.write_text(json.dumps(sample_cases), encoding='utf-8')

            loaded_direct = front.load_case_manifest(manifest_path)
            loaded_sibling = front.load_case_manifest(com_path)
            self.assertEqual(len(loaded_direct), 1)
            self.assertEqual(loaded_direct, loaded_sibling)
            self.assertEqual(loaded_direct[0]['expected_ecx'], 0x24680012)

            # Unrelated binaries in the same directory must never load cases.json
            self.assertEqual(front.load_case_manifest(unrelated_path), [])
            self.assertEqual(front.load_case_manifest(small_path), [])

            # A mismatched/fake CPUTEST.COM must also be rejected
            com_path.write_bytes(b'\x90' * 4000)
            self.assertEqual(front.load_case_manifest(com_path), [])

            raw_bytes, meta = front.build_monitor_case(loaded_direct[0])
            self.assertLessEqual(len(raw_bytes), front.MAX_MONITOR_BYTES)
            self.assertIn('[setup]', front.format_case_inspector(meta, 0x100))
            self.assertIn('[AT TEST INSTRUCTION]', front.format_case_inspector(meta, meta['test_eip']))
            self.assertIn('[checking]', front.format_case_inspector(meta, meta['check_eip']))
            self.assertIn('[PASSED]', front.format_case_inspector(meta, meta['pass_eip']))
            self.assertIn('[FAILED]', front.format_case_inspector(meta, meta['fail_eip']))

            bad_path = Path(d) / 'bad.json'
            bad_path.write_text(json.dumps([{'name': 'broken'}]), encoding='utf-8')
            self.assertEqual(front.load_case_manifest(bad_path), [])

    def test_gui_case_selection_while_busy_and_unrelated_start_regression(self):
        try:
            import tkinter as tk
            root = tk.Tk()
            root.withdraw()
        except Exception as exc:
            self.skipTest(f'Tk display unavailable: {exc}')

        dosbox_mod = self._dosbox_module()
        all_cases = {c['name']: c for c in dosbox_mod.cases()}
        two_cases = [dict(all_cases['add8_0_c0']), dict(all_cases['xchg32_0'])]
        dosbox_mod.build(two_cases)
        engine = front.find_engine()

        def pump_until(predicate, timeout=5.0):
            deadline = time.monotonic() + timeout
            while time.monotonic() < deadline:
                app.poll()
                root.update_idletasks()
                if predicate():
                    return True
                time.sleep(0.01)
            return predicate()

        with tempfile.TemporaryDirectory() as d:
            manifest_path = Path(d) / 'cases.json'
            manifest_path.write_text(json.dumps(two_cases), encoding='utf-8')
            unrelated_path = Path(d) / 'unrelated.bin'
            unrelated_path.write_bytes(b'\x90' * 4000)

            app = front.make_app(root, engine, str(manifest_path))
            try:
                self.assertTrue(pump_until(lambda: not app.busy))
                self.assertEqual(app.active_case_meta['name'], 'add8_0_c0')
                self.assertEqual(app.case_var.get(), '001 add8_0_c0')

                # Put an engine command in flight for case A, then select case B while busy
                self.assertTrue(app.send('step'))
                self.assertTrue(app.busy)
                app.case_combo.current(1)
                app.on_case_selected()

                self.assertTrue(pump_until(lambda: not app.busy and app._pending_case_index is None))
                self.assertEqual(app.case_var.get(), '002 xchg32_0')
                self.assertEqual(app.active_case_meta['name'], 'xchg32_0')
                self.assertIn('XCHG EAX,ECX', app.state['listing'])
                self.assertTrue(app.send('run 256'))
                self.assertTrue(pump_until(lambda: not app.busy))
                self.assertEqual(app.state['registers'][0], 0)
                self.assertEqual(app.state['eip'], app.active_case_meta['pass_eip'] + 6)

                # Starting an oversized unrelated binary beside cases.json must reject it, not load case 1
                app.start(str(unrelated_path))
                self.assertTrue(pump_until(lambda: not app.busy))
                self.assertEqual(app.cases, [])
                self.assertIsNone(app.active_case_meta)
                self.assertEqual(app.status.get(), 'Disconnected')
            finally:
                if not app.closing:
                    app.close()

    def test_std_and_cld_cases_and_mutation_controls(self):
        dosbox_mod = self._dosbox_module()
        all_cases = {c['name']: c for c in dosbox_mod.cases()}
        engine = front.find_engine()

        def run_case(case_dict):
            raw_bytes, meta = front.build_monitor_case(case_dict)
            with tempfile.NamedTemporaryFile(suffix='.bin', delete=False) as tmp:
                tmp.write(raw_bytes)
                tmp_path = Path(tmp.name)
            client = front.EngineClient([engine], tmp_path)
            try:
                client.events.get(timeout=5)
                self.assertTrue(client.request('run 256'))
                _, _, msg = client.events.get(timeout=5)
                return msg, meta
            finally:
                client.close()
                tmp_path.unlink(missing_ok=True)

        for name, bad_opcodes in [
            ('std_arith0', ['90', 'fc', 'f9']),
            ('std_arith1', ['90', 'fc', 'f8']),
            ('cld_arith0', ['90', 'fd', 'f9']),
            ('cld_arith1', ['90', 'fd', 'f8']),
        ]:
            good = dict(all_cases[name], index=77)
            msg, meta = run_case(good)
            self.assertEqual(msg['status'], 'halt', name)
            self.assertEqual(msg['state']['registers'][0], 0, f'{name} should pass')
            self.assertEqual(msg['state']['eip'], meta['pass_eip'] + 6)

            for mutated_hex in bad_opcodes:
                mutated = dict(good, code=mutated_hex)
                m_msg, m_meta = run_case(mutated)
                self.assertEqual(m_msg['status'], 'halt')
                self.assertEqual(
                    m_msg['state']['registers'][0], 77,
                    f'{name} mutated to {mutated_hex} must fail',
                )
                self.assertEqual(m_msg['state']['eip'], m_meta['fail_eip'] + 6)

    def test_monitor_case_executes_and_bisects_in_engine_client(self):
        dosbox_mod = self._dosbox_module()
        all_cases = {c['name']: c for c in dosbox_mod.cases()}
        engine = front.find_engine()

        with tempfile.TemporaryDirectory() as d:
            for name in ('add8_0_c0', 'shl16_1', 'xchg32_0', 'imul32_0', 'lea32_esiedi'):
                case = dict(all_cases[name], index=42)
                raw_bytes, meta = front.build_monitor_case(case)
                bin_path = Path(d) / f'{name}.bin'
                bin_path.write_bytes(raw_bytes)
                client = front.EngineClient([engine], bin_path)
                try:
                    ev, _, msg = client.events.get(timeout=5)
                    self.assertEqual(ev, 'reply')
                    self.assertTrue(client.request(f"break 0x{meta['test_eip']:x}"))
                    client.events.get(timeout=5)
                    self.assertTrue(client.request('run 256'))
                    _, _, bp_msg = client.events.get(timeout=5)
                    self.assertEqual(bp_msg['status'], 'breakpoint')
                    self.assertEqual(bp_msg['state']['eip'], meta['test_eip'])
                    self.assertTrue(client.request('pilstep'))
                    _, _, ps_msg = client.events.get(timeout=5)
                    self.assertEqual(ps_msg['status'], 'stepped')
                    self.assertTrue(client.request('run 256'))
                    _, _, end_msg = client.events.get(timeout=5)
                    self.assertEqual(end_msg['status'], 'halt')
                    self.assertEqual(end_msg['state']['registers'][0], 0)
                finally:
                    client.close()

            # Negative control: corrupt expected_eax and verify monitor halts at fail_eip with EAX=index
            bad_case = dict(all_cases['add8_0_c0'], index=99)
            bad_case['expected_eax'] ^= 1
            bad_bytes, bad_meta = front.build_monitor_case(bad_case)
            bad_path = Path(d) / 'bad.bin'
            bad_path.write_bytes(bad_bytes)
            client = front.EngineClient([engine], bad_path)
            try:
                client.events.get(timeout=5)
                self.assertTrue(client.request('run 256'))
                _, _, fail_msg = client.events.get(timeout=5)
                self.assertEqual(fail_msg['status'], 'halt')
                self.assertEqual(fail_msg['state']['registers'][0], 99)
                self.assertEqual(fail_msg['state']['eip'], bad_meta['fail_eip'] + 6)
            finally:
                client.close()

if __name__=='__main__':unittest.main()
