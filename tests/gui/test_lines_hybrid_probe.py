"""Offline checks for the LINES hybrid acceptance helper (no DOS binary bundled)."""
import hashlib
import importlib.util
from pathlib import Path
import struct
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "verify_lines_hybrid", ROOT / "tools" / "verify_lines_hybrid.py")
helper = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(helper)


class LinesHybridProbeTests(unittest.TestCase):
    def test_lzexe_mz_header_and_sha256(self):
        with tempfile.TemporaryDirectory() as folder:
            data = bytearray(64)
            data[:2] = b"MZ"
            for offset, value in ((2, 64), (4, 1), (8, 2), (20, 14),
                                  (22, 0x1234), (14, 0x20), (16, 0x100)):
                struct.pack_into("<H", data, offset, value)
            data[28:32] = b"LZ91"
            path = Path(folder) / "LINES.EXE"
            path.write_bytes(data)
            info = helper.inspect_mz(path)
            self.assertTrue(info["packed_with_lzexe"])
            self.assertEqual(info["size"], 64)
            self.assertEqual(info["mz_declared"], 64)
            self.assertEqual(info["cs"], 0x1234)
            self.assertEqual(info["sha256"], hashlib.sha256(data).hexdigest())

    def test_bad_mz_size_is_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "BAD.EXE"
            path.write_bytes(b"MZ" + bytes(62))
            with self.assertRaises(ValueError):
                helper.inspect_mz(path)

    def test_snapshot_hashes_pixels_but_preserves_status_and_cpu(self):
        frame = {"status": "waiting_input", "eip": 0x100,
                 "screen": "00abff", "graphics_planar": "cafebeef"}
        original = frame.copy()
        state = helper.summary(frame)
        self.assertEqual(state["status"], "waiting_input")
        self.assertEqual(state["eip"], 0x100)
        self.assertEqual(state["screen"]["chars"], 6)
        self.assertEqual(state["graphics_planar"]["sha256"],
                         hashlib.sha256(b"cafebeef").hexdigest())
        self.assertEqual(frame, original)


if __name__ == "__main__":
    unittest.main()
