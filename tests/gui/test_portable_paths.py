"""Validation must not depend on a writable optional git cache."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import MagicMock, patch

spec = importlib.util.spec_from_file_location(
    'portable_paths', Path(__file__).resolve().parents[2] / 'tools/check_portable_paths.py')
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)


class PortablePathsTests(unittest.TestCase):
    def run_scan(self, text, stamp):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'example.md').write_text(text, encoding='utf-8')
            with patch.object(checker, 'ROOT', root), patch.object(
                    checker, 'files_to_check', return_value=(
                        ['example.md'], 'full', 'revision', 'digest', stamp)), \
                    contextlib.redirect_stdout(io.StringIO()), \
                    contextlib.redirect_stderr(io.StringIO()):
                checker.check(full=True)

    def test_success_updates_writable_cache(self):
        with tempfile.TemporaryDirectory() as directory:
            stamp = Path(directory) / 'stamp'
            self.run_scan('driveD/example.txt', stamp)
            self.assertEqual(json.loads(stamp.read_text()),
                             {'head': 'revision', 'script_sha256': 'digest'})

    def test_success_without_writable_cache(self):
        stamp = MagicMock()
        stamp.write_text.side_effect = PermissionError('read-only metadata')
        self.run_scan('driveD/example.txt', stamp)
        stamp.write_text.assert_called_once()

    def test_bad_paths_fail_before_cache_update(self):
        for writable in (True, False):
            with self.subTest(writable=writable):
                stamp = MagicMock()
                if not writable:
                    stamp.write_text.side_effect = PermissionError('read-only metadata')
                # Construct a synthetic prohibited path without embedding one.
                text = '/'.join(['', 'home', 'example-person', 'checkout'])
                with self.assertRaises(SystemExit):
                    self.run_scan(text, stamp)
                stamp.write_text.assert_not_called()
