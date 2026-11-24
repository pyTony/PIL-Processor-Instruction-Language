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
        for path in ["src/dos.rs", "src/dos/services.rs", "src/pc/ports.rs", "tests/unit/dos.rs", "programs/dos/fixture.asm"]:
            selected = ci.classify([path])
            self.assertEqual(selected, {"dos", "frontend", "integration", "library"})
            self.assertNotIn("core", selected)
            self.assertNotIn("backends", selected)
        self.assertIn("core", ci.classify(["src/dos/services.rs", "src/guest.rs"]))

    def test_delivery(self):
        for path in ["tools/package.py", "tools/export_runtime.py", "tests/gui/test_export_runtime.py", "tests/gui/test_build_dine.py"]:
            self.assertEqual(ci.classify([path, "tools/termux_web.html"]), {"frontend", "web", "delivery"})

    def test_runtime_launchers_do_not_select_cpu_oracles_or_benchmarks(self):
        for path in ci.RUNTIME_LAUNCHERS:
            with self.subTest(path=path):
                self.assertEqual(ci.classify([path]), {"frontend", "library", "delivery"})
        self.assertEqual(ci.classify(["tests/ci/test_runtime_startup.py"]), set())

    def test_rendering_and_containers_keep_dos_coverage_without_cpu_suites(self):
        for path in ci.DOS_SUPPORT:
            with self.subTest(path=path):
                self.assertEqual(ci.classify([path]), {"dos", "frontend", "integration", "library"})
        workflow = (ROOT / ".github/workflows/full.yml").read_text(encoding="utf-8")
        self.assertIn("cargo test --locked containers:: -- --show-output", workflow)

    def test_startup_pr_selects_only_its_affected_modules(self):
        paths = ["src/containers.rs", "src/host/session.rs", "src/host/setup.rs",
                 "src/dine_window.rs", "src/dos/live_diagnostics.rs",
                 "tests/unit/containers.rs", "tools/runtime_startup.py",
                 "tests/ci/test_runtime_startup.py", "docs/DOS_CONFIG.md"]
        self.assertEqual(ci.classify(paths), {"dos", "frontend", "integration", "library", "delivery"})
        # Adding a real decoder/compiler change must restore CPU verification.
        for path in ["src/guest.rs", "src/emit.rs", "src/precompiled/forms.rs", "isa/i386/add.pil"]:
            with self.subTest(path=path):
                selected = ci.classify([*paths, path])
                self.assertTrue({"core", "backends"}.issubset(selected))

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
