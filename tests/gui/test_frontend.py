import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
spec=importlib.util.spec_from_file_location('pil_frontend',Path(__file__).resolve().parents[2]/'tools/pil_debugger.py')
front=importlib.util.module_from_spec(spec);spec.loader.exec_module(front)

class FrontendTests(unittest.TestCase):
    def message(self):
        state={key:'' for key in ['version','demo','listing','memory','stack','segments','cache']}
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

if __name__=='__main__':unittest.main()
