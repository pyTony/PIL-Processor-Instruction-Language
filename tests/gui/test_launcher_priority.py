"""Desktop launchers must not choose an old root engine over a Cargo build."""
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]


class LauncherEnginePriorityTests(unittest.TestCase):
    def test_windows_launcher_prefers_release_and_keeps_override(self):
        text = (ROOT / "PILdos.vbs").read_text(encoding="utf-8")
        release = 'dir & "\\target\\release\\pil.exe"'
        debug = 'dir & "\\target\\debug\\pil.exe"'
        fallback = 'dir & "\\pil.exe"'
        candidates = next(line for line in text.splitlines() if "For Each candidate In Array(" in line)
        self.assertLess(candidates.index(release), candidates.index(debug))
        self.assertLess(candidates.index(debug), candidates.index(fallback))
        self.assertIn('Environment("PROCESS")("PILDOS_ENGINE")', text)
        self.assertIn("If Len(engine) = 0 Then", text)

    def test_shell_launcher_prefers_release_and_keeps_override(self):
        text = (ROOT / "tools/PILdos.sh").read_text(encoding="utf-8")
        candidates = next(line.strip() for line in text.splitlines()
                          if line.strip().startswith("for candidate in "))
        self.assertEqual(
            candidates,
            "for candidate in ./target/release/pil ./target/debug/pil ./pil; do",
        )
        self.assertIn("engine=${PILDOS_ENGINE:-}", text)
        self.assertIn('if [ -z "$engine" ]; then', text)


if __name__ == "__main__":
    unittest.main()
