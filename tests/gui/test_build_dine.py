"""Build-helper tests: no Rust toolchain or real guest execution required.

Full-system Dine CPU/VGA behavior remains covered by the existing DOS tests;
the actual Windows DLL/EXE startup requires a platform smoke test.
"""
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("build_dine", ROOT / "tools" / "build_dine.py")
build_dine = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(build_dine)


class DineBuildTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.guest = self.root / "MANDVGA.COM"
        self.guest.write_bytes(b"original mandelbrot COM bytes\xcd\x20")
        self.output = self.root / "compiled"
        self.library = self.output / build_dine.library_name()
        name = f"Dine_MANDVGA{'.exe' if build_dine.os.name == 'nt' else ''}"
        self.launcher = self.output / name
        self.manifest = self.output / (name + ".json")
        self.calls = []
        self.fail_rustc = False

    def invoke(self, force=False):
        def fake_run(argv, **kwargs):
            self.calls.append((argv, kwargs))
            self.assertNotIn("PIL_SHADOW_SOURCE", kwargs["env"])
            self.assertNotIn("CARGO_BUILD_TARGET", kwargs["env"])
            if argv[0] == "cargo":
                self.assertIn("--lib", argv)
                self.assertIn("dine-library", argv)
                path = self.root / "target" / "release" / build_dine.library_name()
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"shared Dine runtime, no guest blocks")
            elif argv[0] == "rustc":
                if self.fail_rustc:
                    raise subprocess.CalledProcessError(1, argv)
                Path(argv[-1]).write_bytes(b"small host executable containing guest bytes")
            else:
                self.fail(f"unexpected invocation: {argv!r}")
            return subprocess.CompletedProcess(argv, 0)

        with mock.patch.object(build_dine, "ROOT", self.root), \
             mock.patch.object(build_dine.subprocess, "run", side_effect=fake_run):
            return build_dine.build(self.guest, self.output, force=force)

    def test_builds_small_launcher_with_shared_guest_independent_runtime(self):
        result = self.invoke()
        self.assertTrue(result.samefile(self.launcher),
                        f"builder returned {result}, expected {self.launcher}")
        self.assertEqual(self.library.read_bytes(), b"shared Dine runtime, no guest blocks")
        self.assertEqual(self.launcher.read_bytes(), b"small host executable containing guest bytes")
        record = json.loads(self.manifest.read_text())
        self.assertEqual(record["backend"], "Dine interpreter through shared native library")
        self.assertEqual(record["guest_sha256"], hashlib.sha256(self.guest.read_bytes()).hexdigest())
        self.assertFalse(record["compiled_guest_blocks"])
        self.assertTrue(record["original_guest_embedded"])
        self.assertFalse(record["external_dos_file_required"])
        self.assertEqual([argv[0] for argv, _ in self.calls], ["cargo", "rustc"])
        source = self.root / "target" / "dine-launchers" / "MANDVGA" / "main.rs"
        body = source.read_text()
        self.assertIn("include_bytes!", body)
        self.assertIn(str(self.guest.resolve()), body)
        self.assertIn("dine_abi_version", body)
        self.assertIn("dine_run", body)

    def test_existing_launcher_is_protected(self):
        self.invoke()
        before = (self.library.read_bytes(), self.launcher.read_bytes())
        with self.assertRaises(FileExistsError):
            self.invoke()
        self.assertEqual((self.library.read_bytes(), self.launcher.read_bytes()), before)

    def test_failed_native_launcher_build_does_not_replace_published_pair(self):
        self.invoke()
        before = (self.library.read_bytes(), self.launcher.read_bytes())
        self.fail_rustc = True
        with self.assertRaises(subprocess.CalledProcessError):
            self.invoke(force=True)
        self.assertEqual((self.library.read_bytes(), self.launcher.read_bytes()), before)

    def test_rejects_non_dos_and_empty_inputs(self):
        with self.assertRaises(ValueError):
            self.invoke_invalid(self.root / "not-dos.bin", b"data")
        with self.assertRaises(ValueError):
            self.invoke_invalid(self.root / "empty.COM", b"")

    def invoke_invalid(self, path, data):
        path.write_bytes(data)
        with mock.patch.object(build_dine, "ROOT", self.root):
            return build_dine.build(path, self.output)


if __name__ == "__main__":
    unittest.main()
