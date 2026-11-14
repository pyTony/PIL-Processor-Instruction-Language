"""The delivered Rust tree must not depend on unexported development tests."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location(
    "export_runtime", Path(__file__).resolve().parents[2] / "tools" / "export_runtime.py")
exporter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(exporter)


class RuntimeExportTests(unittest.TestCase):
    def test_external_test_paths_are_removed_from_lf_and_crlf_checkouts(self):
        source = ('pub fn entry() {}\n#[cfg(test)]\n'
                  '#[path="../tests/unit/not_delivered.rs"]\nmod tests;\n')
        for newline in ("\n", "\r\n"):
            with self.subTest(newline=repr(newline)):
                result = exporter.runtime_rust(source.replace("\n", newline).encode())
                self.assertEqual(result, b"pub fn entry() {}\n")

    def test_internal_test_items_keep_their_normal_cfg_gate(self):
        source = b"#[cfg(test)]\nfn internal_fixture() {}\n"
        self.assertEqual(exporter.runtime_rust(source), source)
