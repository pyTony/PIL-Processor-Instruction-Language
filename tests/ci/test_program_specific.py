"""Keep system-code checks active in manifests and nested build tools."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "program_specific", ROOT / "tools" / "check_program_specific.py")
checker = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(checker)


class SharedSystemCodeTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        patch = mock.patch.object(checker, "ROOT", self.root)
        patch.start()
        self.addCleanup(patch.stop)

    def write(self, relative, text):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def test_manifest_cannot_select_a_guest_specific_system_build(self):
        self.write("Cargo.toml", '[features]\nquake = []\n')
        self.assertTrue(any(f[0] == "Cargo.toml" for f in checker.scan()))

    def test_nested_general_tools_cannot_special_case_guest_names(self):
        for extension in ("py", "ps1", "sh", "cmd", "bat", "vbs"):
            self.write(f"tools/build/engine.{extension}", 'name = "QUAKE.EXE"\n')
        self.assertEqual({f[0] for f in checker.scan()}, {
            f"tools/build/engine.{ext}" for ext in ("py", "ps1", "sh", "cmd", "bat", "vbs")})

    def test_nested_tools_cannot_restore_removed_per_program_engines(self):
        self.write("tools/build/engine.py", 'env["PIL_' + 'SHADOW_SOURCE"] = source\n')
        with mock.patch.object(checker.subprocess, "run") as git:
            git.return_value.stdout = ""
            findings = checker.per_program_builds()
        self.assertEqual(len(findings), 1, findings)
        self.assertIn("tools/build/engine.py:1: per-program build interface", findings[0])

    def test_guest_specific_fixtures_stay_outside_general_system_code(self):
        self.write("tests/unit/guest.rs", 'const NAME: &str = "QUAKE.EXE";\n')
        self.write("tools/fixtures/verify_guest.py", 'name = "QUAKE.EXE"\n')
        self.write("src/core.rs", '#[cfg(test)]\nmod tests { const NAME: &str = "QUAKE.EXE"; }\n')
        self.assertEqual(checker.scan(), [])


if __name__ == "__main__":
    unittest.main()
