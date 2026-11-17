"""Exercise CI selection boundaries, including mixed and deleted/renamed paths."""
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch
import subprocess

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("ci_changes", ROOT / "tools/ci_changes.py")
ci = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ci)


class ChangesTests(unittest.TestCase):
    def test_docs(self):
        self.assertEqual(ci.classify(["README.md", "docs/SHA256SUMS"]), set())

    def test_frontend(self):
        self.assertEqual(ci.classify(["tools/termux_web.html"]), {"web"})
        self.assertEqual(ci.classify(["tools/dos_terminal.py"]), {"frontend"})

    def test_engine_and_descriptions(self):
        for path in ["src/guest.rs", "src/memory.rs", "isa/i386/add.pil", "Cargo.lock", "tests/unit/cpu.rs", "programs/selftest.hex"]:
            self.assertEqual(ci.classify([path]), {"core", "dos", "frontend", "integration", "library", "backends"})

    def test_dos_changes_do_not_select_instruction_suites(self):
        for path in ["src/dos.rs", "src/dos_services.rs", "src/dos_ports.rs", "tests/unit/dos.rs", "programs/dos/fixture.asm"]:
            selected = ci.classify([path])
            self.assertEqual(selected, {"dos", "frontend", "integration", "library"})
            self.assertNotIn("core", selected)
            self.assertNotIn("backends", selected)
        self.assertIn("core", ci.classify(["src/dos_services.rs", "src/guest.rs"]))

    def test_delivery(self):
        for path in ["tools/package.py", "tools/export_runtime.py", "tests/gui/test_export_runtime.py", "tests/gui/test_build_dine.py"]:
            self.assertEqual(ci.classify([path, "tools/termux_web.html"]), {"frontend", "web", "delivery"})

    def test_unknown_and_ci_fail_closed(self):
        for path in ["new/input.dat", ".github/workflows/check.yml", "tools/ci_changes.py", "tools/verify_dosbox_cpu.py"]:
            self.assertEqual(ci.classify([path]), set(ci.MODULES))

    def test_mixed_changes_accumulate(self):
        self.assertEqual(ci.classify(["src/main.rs", "tools/package.py", "tools/bench.sh", "tools/termux_web.html"]), set(ci.MODULES))

    def test_diff_preserves_both_sides_of_rename(self):
        env = {"PR_BASE": "base", "PR_HEAD": "head"}
        with patch.object(ci.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, b"src/old.rs\0docs/new.md\0")) as run:
            self.assertEqual(ci.changed_paths("pull_request", env), ["src/old.rs", "docs/new.md"])
            self.assertIn("--no-renames", run.call_args.args[0])
            self.assertEqual(run.call_args.args[0][-1], "base...head")

    def test_refresh_compares_entire_branch(self):
        with patch.object(ci.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, b"")) as run:
            ci.changed_paths("workflow_dispatch", {"BASE_REF": "dev"})
            self.assertEqual(run.call_args.args[0][-1], "origin/dev...HEAD")

    def test_manual_without_range_requires_full(self):
        with self.assertRaises(ValueError):
            ci.changed_paths("workflow_dispatch", {})


if __name__ == "__main__":
    unittest.main()
