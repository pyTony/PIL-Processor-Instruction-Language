"""Smoke-check the native DOS build wrapper without an installed Rust toolchain.

Instruction semantics and execution are tested by verify_dos_standalone.py,
verify_factor_native.py, verify_recursive_native.py and verify_mz_native.py.
"""
import contextlib
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("build_dos_native", ROOT / "tools" / "build_dos_native.py")
build_dos_native = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(build_dos_native)


class NativeBuildHelperTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.work = Path(self.temp.name)
        self.guest = self.work / "FACTOR.COM"
        self.guest.write_bytes(b"original DOS image")
        self.engine = self.work / "pil"
        self.engine.write_bytes(b"existing PIL emitter")
        self.out = self.work / "compiled"

    def artifacts(self):
        name = self.guest.name + (".exe" if build_dos_native.os.name == "nt" else ".native")
        return self.out / name, self.out / (name + ".rs"), self.out / (name + ".json")

    def invoke(self, force=False, fail_rustc=False):
        calls = []

        def fake_run(command, **kwargs):
            calls.append(command)
            if "--emit-dos-standalone" in command:
                Path(command[-1]).write_text("fn main() {}\n")
            elif command[0] == "rustc":
                if fail_rustc:
                    raise subprocess.CalledProcessError(1, command)
                Path(command[-1]).write_bytes(b"host native executable")
            else:
                raise AssertionError(f"unexpected command {command!r}")
            return subprocess.CompletedProcess(command, 0)

        argv = ["build_dos_native.py", str(self.guest), "--engine", str(self.engine),
                "--output-dir", str(self.out)]
        if force:
            argv.append("--force")
        with mock.patch.object(sys, "argv", argv), \
             mock.patch.object(build_dos_native.subprocess, "run", side_effect=fake_run), \
             mock.patch.object(build_dos_native.subprocess, "check_output",
                               return_value="rustc 1.99.1\nhost: x86_64-pc-windows-msvc\n"), \
             contextlib.redirect_stdout(io.StringIO()), \
             contextlib.redirect_stderr(io.StringIO()):
            build_dos_native.main()
        return calls

    def test_emits_build_identity_and_native_host_binary(self):
        calls = self.invoke()
        executable, source, manifest = self.artifacts()
        self.assertEqual(executable.read_bytes(), b"host native executable")
        self.assertEqual(source.read_text(), "fn main() {}\n")
        record = json.loads(manifest.read_text())
        self.assertEqual(record["mode"], "standalone")
        self.assertFalse(record["automatic_dos_drive_loading"])
        self.assertEqual(record["guest_sha256"], hashlib.sha256(self.guest.read_bytes()).hexdigest())
        self.assertEqual(record["engine_sha256"], hashlib.sha256(self.engine.read_bytes()).hexdigest())
        self.assertEqual(record["source_sha256"], hashlib.sha256(source.read_bytes()).hexdigest())
        self.assertEqual(record["executable_sha256"], hashlib.sha256(executable.read_bytes()).hexdigest())
        self.assertIn("--emit-dos-standalone", calls[0])
        self.assertEqual(calls[1][0], "rustc")

    def test_refuses_overwrite_without_force(self):
        self.invoke()
        originals = [p.read_bytes() for p in self.artifacts()]
        with self.assertRaises(SystemExit):
            self.invoke()
        self.assertEqual(originals, [p.read_bytes() for p in self.artifacts()])

    def test_failed_compiler_preserves_existing_build(self):
        self.invoke()
        originals = [p.read_bytes() for p in self.artifacts()]
        with self.assertRaises(subprocess.CalledProcessError):
            self.invoke(force=True, fail_rustc=True)
        self.assertEqual(originals, [p.read_bytes() for p in self.artifacts()])


if __name__ == "__main__":
    unittest.main()
