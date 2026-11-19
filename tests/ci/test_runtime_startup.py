"""Shared runtime checks must track system inputs and report Git state."""
import contextlib
import importlib.util
import io
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("runtime_startup", ROOT / "tools/runtime_startup.py")
startup = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(startup)


class RuntimeStartupTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "Cargo.toml").write_text('[package]\nname="fixture"\n')
        (self.root / "src").mkdir()
        self.source = self.root / "src/core.rs"
        self.source.write_text("// first source\n")
        self.output = self.root / "target/release/libdine.so"

    def test_unchanged_system_skips_cargo_and_changed_source_rebuilds(self):
        def build(command, **kwargs):
            self.assertIn("--lib", command)
            self.assertIn("dine-library", command)
            self.output.write_bytes(self.source.read_bytes())
            return subprocess.CompletedProcess(command, 0)
        with mock.patch.object(startup.shutil, "which", return_value="cargo"), \
             mock.patch.object(startup.subprocess, "run", side_effect=build) as run:
            startup.ensure(self.root, self.output, "library")
            startup.ensure(self.root, self.output, "library")
            self.assertEqual(run.call_count, 1)
            self.source.write_text("// second source\n")
            startup.ensure(self.root, self.output, "library")
            self.assertEqual(run.call_count, 2)
        self.assertFalse((self.root / "target/pil-build-in-progress").exists())

    def test_failed_build_clears_lock_and_does_not_mark_runtime_current(self):
        with mock.patch.object(startup.shutil, "which", return_value="cargo"), \
             mock.patch.object(startup.subprocess, "run", side_effect=subprocess.CalledProcessError(1, ["cargo"])):
            with self.assertRaises(subprocess.CalledProcessError):
                startup.ensure(self.root, self.output, "library")
        self.assertFalse(startup.current(self.root, self.output, "library"))
        self.assertFalse((self.root / "target/pil-build-in-progress").exists())

    def test_active_build_prevents_launch_of_older_output(self):
        marker = self.root / "target/pil-build-in-progress"
        marker.mkdir(parents=True)
        with self.assertRaisesRegex(ValueError, "in progress"):
            startup.ensure(self.root, self.output, "library")

    def test_branch_warning_and_status_are_local_read_only_queries(self):
        (self.root / ".git").mkdir()
        for branch in ("dev", "main", "feature/example", "(detached)"):
            result = subprocess.CompletedProcess([], 0, stdout=(
                f"# branch.head {branch}\n# branch.ab +2 -1\n1 tracked change\n? local-file\n"))
            with self.subTest(branch=branch), mock.patch.object(startup.subprocess, "run", return_value=result) as run, \
                 contextlib.redirect_stderr(io.StringIO()) as output:
                startup.git_status(self.root)
            text = output.getvalue()
            self.assertIn("1 changed, 1 untracked", text)
            self.assertIn("+2 -1", text)
            self.assertEqual("not dev/main" in text, branch not in ("dev", "main"))
            self.assertEqual(run.call_args.args[0], ["git", "status", "--porcelain=v2", "--branch"])


if __name__ == "__main__":
    unittest.main()
