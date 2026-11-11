import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]


class ShortcutTests(unittest.TestCase):
    def setUp(self):
        (ROOT / 'target').mkdir(exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(prefix='shortcut-', dir=ROOT / 'target')
        self.addCleanup(self.temp.cleanup)
        self.environment = patch.dict(os.environ, {'PILDOS_CONFIG': '', 'PILDRIVE': 'driveD'})
        self.environment.start()
        self.addCleanup(self.environment.stop)

    def checkout(self, name):
        root = Path(self.temp.name) / name
        (root / 'tools').mkdir(parents=True)
        shutil.copyfile(ROOT / 'tools/makeshortcut.py', root / 'tools/makeshortcut.py')
        for name in ('PILdos.vbs', 'PILdos.sh', 'PILdos.ico', 'tools/dos_terminal.py', 'tools/pil_debugger.py'):
            (root / name).write_bytes(b'fixture')
        spec = importlib.util.spec_from_file_location('shortcut_fixture', root / 'tools/makeshortcut.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def engine(self, module):
        folder = module.root / 'target/release'
        folder.mkdir(parents=True)
        (folder / ('pil.exe' if sys.platform == 'win32' else 'pil')).write_bytes(b'fixture')

    def test_missing_engine_explains_how_to_build_in_this_checkout(self):
        module = self.checkout('renamed release')
        with self.assertRaisesRegex(SystemExit, 'cargo build --release --locked'):
            module.validate_setup()

    def test_missing_default_data_is_reported_before_creating_shortcut(self):
        module = self.checkout('pil_main')
        self.engine(module)
        with self.assertRaisesRegex(SystemExit, 'FD14LITE.img'):
            module.validate_setup()
        (module.root / 'images').mkdir()
        (module.root / 'images/FD14LITE.img').write_bytes(b'fixture')
        (module.root / ('DriveD' if sys.platform == 'win32' else 'driveD')).mkdir()
        module.validate_setup()

    def test_custom_config_does_not_require_default_data_paths(self):
        module = self.checkout('external data')
        self.engine(module)
        (module.root / 'pildos.conf').write_text('program=elsewhere/hello.com\n')
        module.validate_setup()

    def test_relative_config_uses_checkout_not_callers_working_directory(self):
        module = self.checkout('another folder')
        self.engine(module)
        (module.root / 'custom.conf').write_text('program=elsewhere/hello.com\n')
        with patch.dict(os.environ, {'PILDOS_CONFIG': 'custom.conf'}):
            module.validate_setup()
        with patch.dict(os.environ, {'PILDOS_CONFIG': 'missing.conf'}):
            with self.assertRaisesRegex(SystemExit, 'PILDOS_CONFIG does not exist'):
                module.validate_setup()

    @unittest.skipUnless(sys.platform == 'win32' and shutil.which('powershell.exe'), 'Windows shortcuts require PowerShell')
    def test_real_shortcut_tracks_renamed_checkout_with_spaces_and_quotes(self):
        desktop = Path(self.temp.name) / 'desktop'
        desktop.mkdir()
        for name in ('pil_main', "Release build's folder"):
            module = self.checkout(name)
            # Redirect only the output folder; exercise the generated PowerShell
            # and actual WScript shortcut persistence without touching Desktop.
            with patch.object(module.subprocess, 'run') as run:
                module.windows()
            script = run.call_args.args[0][-1]
            destination = "'" + str(desktop).replace("'", "''") + "'"
            script = script.replace("[Environment]::GetFolderPath('Desktop')", destination)
            subprocess.run(['powershell.exe', '-NoProfile', '-Command', script], check=True, capture_output=True)
            path = str(desktop / 'PIL DOS.lnk').replace("'", "''")
            inspect = "$s=(New-Object -ComObject WScript.Shell).CreateShortcut('" + path + "'); @{target=$s.TargetPath;directory=$s.WorkingDirectory;icon=$s.IconLocation} | ConvertTo-Json -Compress"
            result = subprocess.run(['powershell.exe', '-NoProfile', '-Command', inspect], check=True, capture_output=True, text=True)
            link = json.loads(result.stdout.lstrip('\ufeff'))
            self.assertEqual(Path(link['target']), module.root / 'PILdos.vbs')
            self.assertEqual(Path(link['directory']), module.root)
            self.assertEqual(Path(link['icon'].rsplit(',', 1)[0]), module.root / 'PILdos.ico')


if __name__ == '__main__':
    unittest.main()
