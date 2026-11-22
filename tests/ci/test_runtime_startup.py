"""Shared runtime checks must track system inputs and report Git state."""
import contextlib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
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

    def test_gui_build_shows_progress_but_current_runtime_stays_quiet(self):
        def build(command, **kwargs):
            self.output.write_bytes(self.source.read_bytes())
            return subprocess.CompletedProcess(command, 0)
        with mock.patch.object(startup.shutil, "which", return_value="cargo"), \
             mock.patch.object(startup.subprocess, "run", side_effect=build), \
             mock.patch.object(startup, "show_build_progress") as show:
            startup.ensure(self.root, self.output, "library", gui=True)
            startup.ensure(self.root, self.output, "library", gui=True)
            self.assertEqual(show.call_count, 1)
            self.source.write_text("// changed source\n")
            startup.ensure(self.root, self.output, "library", gui=True)
            self.assertEqual(show.call_count, 2)

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

    def test_publish_latest_updates_pointer_and_prunes_older_unlocked_builds(self):
        def build(command, **kwargs):
            self.output.write_bytes(self.source.read_bytes())
            return subprocess.CompletedProcess(command, 0)
        with mock.patch.object(startup.shutil, "which", return_value="cargo"), \
             mock.patch.object(startup.subprocess, "run", side_effect=build):
            startup.ensure(self.root, self.output, "library")
            first_sha = hashlib.sha256(self.output.read_bytes()).hexdigest()
            latest_file = self.output.parent / "dine.latest.json"
            first_record = json.loads(latest_file.read_text(encoding="utf-8"))
            self.assertEqual(first_record["shared_runtime"], self.output.name)
            self.assertFalse(Path(first_record["shared_runtime_dir"]).is_absolute())
            self.assertEqual(first_record["shared_runtime_dir"], f"../runtime-builds/{first_sha[:16]}")
            self.assertEqual(first_record["sha256"], first_sha)
            first_copy = (self.output.parent / first_record["shared_runtime_dir"] / first_record["shared_runtime"]).resolve()
            self.assertTrue(first_copy.is_file())

            self.source.write_text("// second build\n")
            startup.ensure(self.root, self.output, "library")
            second_sha = hashlib.sha256(self.output.read_bytes()).hexdigest()
            self.assertNotEqual(first_sha, second_sha)
            second_record = json.loads(latest_file.read_text(encoding="utf-8"))
            self.assertEqual(second_record["shared_runtime_dir"], f"../runtime-builds/{second_sha[:16]}")
            self.assertTrue((self.root / "target/runtime-builds" / second_sha[:16] / self.output.name).is_file())
            self.assertFalse((self.root / "target/runtime-builds" / first_sha[:16]).exists())

    def test_cli_adopts_newer_manual_build_and_refreshes_stamp(self):
        cli = self.root / "target/release" / ("pil.exe" if os.name == "nt" else "pil")

        def build(command, **kwargs):
            cli.parent.mkdir(parents=True, exist_ok=True)
            cli.write_bytes(self.source.read_bytes())
            return subprocess.CompletedProcess(command, 0)

        with mock.patch.object(startup.shutil, "which", return_value="cargo"), \
             mock.patch.object(startup.subprocess, "run", side_effect=build) as run:
            startup.ensure(self.root, cli, "cli")
            self.assertEqual(run.call_count, 1)

            stamp_file = startup.stamp_path(self.root, cli)
            t0 = max(stamp_file.stat().st_mtime_ns, (self.root / "Cargo.toml").stat().st_mtime_ns, cli.stat().st_mtime_ns) + 10_000_000
            os.utime(stamp_file, ns=(t0, t0))
            self.source.write_text("// manual source change\n")
            os.utime(self.source, ns=(t0 + 1_000_000, t0 + 1_000_000))
            cli.write_bytes(b"manually rebuilt cli binary")
            os.utime(cli, ns=(t0 + 2_000_000, t0 + 2_000_000))

            startup.ensure(self.root, cli, "cli")
            self.assertEqual(run.call_count, 1)
            refreshed = json.loads(stamp_file.read_text(encoding="utf-8"))
            self.assertEqual(refreshed["output"], hashlib.sha256(cli.read_bytes()).hexdigest())
            self.assertEqual(refreshed["inputs"], startup.fingerprint(self.root))

            self.source.write_text("// edited without rebuilding\n")
            os.utime(self.source, ns=(t0 + 3_000_000, t0 + 3_000_000))
            startup.ensure(self.root, cli, "cli")
            self.assertEqual(run.call_count, 2)

    def test_stale_build_marker_with_dead_pid_is_recovered(self):
        marker = self.root / "target/pil-build-in-progress"
        marker.mkdir(parents=True)
        (marker / "pid").write_text("999999\n", encoding="utf-8")

        def build(command, **kwargs):
            self.output.parent.mkdir(parents=True, exist_ok=True)
            self.output.write_bytes(self.source.read_bytes())
            return subprocess.CompletedProcess(command, 0)

        with mock.patch.object(startup, "_pid_alive", return_value=True):
            with self.assertRaisesRegex(ValueError, "in progress"):
                startup.ensure(self.root, self.output, "library")
        self.assertTrue(marker.exists())

        with mock.patch.object(startup, "_pid_alive", return_value=False), \
             mock.patch.object(startup.shutil, "which", return_value="cargo"), \
             mock.patch.object(startup.subprocess, "run", side_effect=build):
            startup.ensure(self.root, self.output, "library")
        self.assertTrue(self.output.is_file())
        self.assertFalse(marker.exists())

    def test_locked_output_fails_fast_with_clear_message(self):
        self.output.parent.mkdir(parents=True, exist_ok=True)
        self.output.write_bytes(b"locked")
        with mock.patch.object(startup, "_file_in_use", return_value=True), \
             mock.patch.object(startup.shutil, "which", return_value="cargo"), \
             mock.patch.object(startup.subprocess, "run") as run:
            with self.assertRaisesRegex(ValueError, "in use"):
                startup.ensure(self.root, self.output, "library")
        run.assert_not_called()

    def test_main_writes_and_clears_last_error(self):
        marker = self.root / "target/pil-build-in-progress"
        marker.mkdir(parents=True)
        argv = ["runtime_startup.py", "--root", str(self.root), "--engine", str(self.output), "--kind", "library"]
        with mock.patch.object(sys, "argv", argv), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(startup.main(), 1)
        err_file = startup.last_error_path(self.root)
        self.assertTrue(err_file.is_file())
        self.assertIn("in progress", err_file.read_text(encoding="utf-8"))

        marker.rmdir()

        def build(command, **kwargs):
            self.output.parent.mkdir(parents=True, exist_ok=True)
            self.output.write_bytes(self.source.read_bytes())
            return subprocess.CompletedProcess(command, 0)

        with mock.patch.object(sys, "argv", argv), \
             mock.patch.object(startup.shutil, "which", return_value="cargo"), \
             mock.patch.object(startup.subprocess, "run", side_effect=build), \
             contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(startup.main(), 0)
        self.assertFalse(err_file.exists())


if __name__ == "__main__":
    unittest.main()
