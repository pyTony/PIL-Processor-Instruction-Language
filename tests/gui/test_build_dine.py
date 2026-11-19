"""Build-helper tests: no Rust toolchain or real guest execution required.

Full-system Dine CPU/VGA behavior remains covered by the existing DOS tests;
the actual Windows DLL/EXE startup requires a platform smoke test.
"""
import hashlib
import importlib.util
import contextlib
import io
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
        (self.root / "Cargo.toml").write_text('[package]\nname="fixture"\nversion="0.1.0"\n')
        self.guest = self.root / "MANDVGA.COM"
        self.guest.write_bytes(b"original mandelbrot COM bytes\xcd\x20")
        self.output = self.root / "compiled"
        self.library = self.output / build_dine.library_name()
        name = f"Dine_MANDVGA{'.exe' if build_dine.os.name == 'nt' else ''}"
        self.launcher = self.output / name
        self.manifest = self.output / (name + ".json")
        self.calls = []
        self.fail_rustc = False
        self.run_exit = 0
        self.comparison_mismatch = False

    def invoke(self, force=False, argv=None, tools_available=True, cargo_available=True):
        def fake_run(argv, **kwargs):
            self.calls.append((argv, kwargs))
            if "env" in kwargs:
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
            elif Path(argv[0]).name.startswith("Dine_"):
                if kwargs.get("capture_output"):
                    elapsed = "20.000" if kwargs["env"].get("PIL_PRECOMPILED") == "off" else "10.000"
                    instructions = 124 if self.comparison_mismatch and kwargs["env"].get("PIL_PRECOMPILED") == "on" else 123
                    return subprocess.CompletedProcess(argv, self.run_exit,
                        stdout=f"DINE_TIMING elapsed_ms={elapsed} instructions={instructions} boundary=waiting_input presentation=off\n", stderr="")
                return subprocess.CompletedProcess(argv, self.run_exit)
            else:
                self.fail(f"unexpected invocation: {argv!r}")
            return subprocess.CompletedProcess(argv, 0)

        with mock.patch.object(build_dine, "ROOT", self.root), \
             mock.patch.object(build_dine.shutil, "which", side_effect=lambda tool: tool if tools_available and (tool != "cargo" or cargo_available) else None), \
             mock.patch.object(build_dine.subprocess, "run", side_effect=fake_run):
            if argv is not None:
                return build_dine.main(argv)
            return build_dine.build(self.guest, self.output, force=force)

    def test_builds_small_launcher_with_shared_guest_independent_runtime(self):
        result = self.invoke()
        self.assertTrue(result.samefile(self.launcher),
                        f"builder returned {result}, expected {self.launcher}")
        self.assertEqual(self.library.read_bytes(), b"shared Dine runtime, no guest blocks")
        self.assertEqual(self.launcher.read_bytes(), b"small host executable containing guest bytes")
        record = json.loads(self.manifest.read_text())
        self.assertEqual(record["backend"], "Shared PIL/Dine native forms with interpreter fallback")
        self.assertEqual(record["guest_sha256"], hashlib.sha256(self.guest.read_bytes()).hexdigest())
        self.assertFalse(record["compiled_guest_blocks"])
        self.assertTrue(record["original_guest_embedded"])
        self.assertFalse(record["external_dos_file_required"])
        self.assertEqual([argv[0] for argv, _ in self.calls], ["cargo", "rustc"])
        source = self.root / "target" / "dine-launchers" / "MANDVGA" / "main.rs"
        body = source.read_text()
        self.assertIn("include_bytes!", body)
        self.assertNotIn(str(self.guest.resolve()), body)
        self.assertIn('include_bytes!(r#"guest.bin"#)', body)
        self.assertEqual(source.with_name("guest.bin").read_bytes(), self.guest.read_bytes())
        self.assertIn("dine_abi_version", body)
        self.assertIn("dine_run", body)

    def test_launcher_contains_no_copy_of_dine_or_pil(self):
        # Tony, 13 November 2026: launchers are fine "as long as they contain
        # not dine or pil". The launcher is standard-library Rust plus the
        # guest bytes; Dine and PIL come only from the shared library at run
        # time.
        self.invoke()
        body = (self.root / "target" / "dine-launchers" / "MANDVGA" / "main.rs").read_text()
        for forbidden in ("include!(", "#[path", "extern crate", "mod ", "pil::", "dine::",
                          "isa/", "src/"):
            self.assertNotIn(forbidden, body.replace(str(self.guest.resolve()), ""), forbidden)
        self.assertEqual(body.count("include_bytes!"), 1)
        rustc = next(argv for argv, _ in self.calls if argv[0] == "rustc")
        self.assertFalse(any(a in ("--extern", "-l", "-L") or a.startswith(("--extern", "-l", "-L"))
                             for a in rustc), rustc)

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

    def test_cli_batch_builds_one_shared_runtime_for_multiple_inputs(self):
        other = self.root / "SECOND.COM"
        other.write_bytes(b"\xcd\x20")
        self.assertEqual(self.invoke(argv=[str(self.guest), str(other),
                                          "--output-dir", str(self.output)]), 0)
        self.assertEqual([argv[0] for argv, _ in self.calls], ["cargo", "rustc", "rustc"])
        self.assertTrue((self.output / build_dine.launcher_name(other)).is_file())
        cargo, options = self.calls[0]
        self.assertNotIn(str(self.guest), cargo)
        self.assertNotIn(str(other), cargo)
        self.assertNotIn(str(self.guest), options["env"].values())

    def test_cli_run_preserves_arguments_assets_directory_and_exit_status(self):
        self.run_exit = 17
        result = self.invoke(argv=[str(self.guest), "--output-dir", str(self.output),
                                   "--run", "--", "source file.pas", "--force"])
        self.assertEqual(result, 17)
        command, options = self.calls[-1]
        self.assertTrue(Path(command[0]).samefile(self.launcher))
        self.assertEqual(command[1:], ["source file.pas", "--force"])
        # Windows may expand a short-path alias when resolving the run folder.
        self.assertTrue(Path(options["cwd"]).samefile(self.guest.parent))

    def test_cli_reuses_prebuilt_runtime_without_invoking_cargo(self):
        runtime = self.root / "shared" / build_dine.library_name()
        runtime.parent.mkdir()
        runtime.write_bytes(b"prebuilt shared system")
        self.assertEqual(self.invoke(argv=[str(self.guest), "--runtime", str(runtime),
                                          "--output-dir", str(self.output)]), 0)
        self.assertEqual([argv[0] for argv, _ in self.calls], ["rustc"])
        self.assertEqual(self.library.read_bytes(), runtime.read_bytes())

    def test_cli_rejects_colliding_outputs_before_building(self):
        other = self.root / "elsewhere" / self.guest.name
        other.parent.mkdir()
        other.write_bytes(b"\xcd\x20")
        with contextlib.redirect_stderr(io.StringIO()) as errors:
            self.assertEqual(self.invoke(argv=[str(self.guest), str(other)]), 1)
        self.assertIn("same output name", errors.getvalue())
        self.assertEqual(self.calls, [])

    def test_cli_missing_tools_reports_setup_error_without_traceback(self):
        with mock.patch.object(build_dine.shutil, "which", return_value=None), \
             mock.patch.object(build_dine, "ROOT", self.root), \
             mock.patch.object(build_dine.subprocess, "run") as run, \
             contextlib.redirect_stderr(io.StringIO()) as errors:
            self.assertEqual(build_dine.main([str(self.guest), "--output-dir", str(self.output)]), 1)
        self.assertIn("--doctor", errors.getvalue())
        run.assert_not_called()

    def test_cli_doctor_does_not_build_or_require_a_guest(self):
        with mock.patch.object(build_dine.shutil, "which", side_effect=lambda tool: tool), \
             mock.patch.object(build_dine.subprocess, "run") as run:
            self.assertEqual(build_dine.main(["--doctor"]), 0)
        self.assertEqual([c.args[0] for c in run.call_args_list],
                         [["rustc", "--version"], ["cargo", "--version"]])

    def test_cli_does_not_run_ambiguous_batches_or_silently_discard_arguments(self):
        for args in ([str(self.guest), str(self.guest), "--run"],
                     [str(self.guest), "--", "guest-argument"],
                     [str(self.guest), "--working-dir", str(self.root)]):
            with self.subTest(args=args), contextlib.redirect_stderr(io.StringIO()), \
                 self.assertRaises(SystemExit) as error:
                self.invoke(argv=args)
            self.assertEqual(error.exception.code, 2)
        self.assertEqual(self.calls, [])

    def test_cli_run_accepts_an_explicit_data_directory(self):
        data = self.root / "data with spaces"
        data.mkdir()
        self.assertEqual(self.invoke(argv=[str(self.guest), "--output-dir", str(self.output),
                                          "--run", "--working-dir", str(data)]), 0)
        self.assertTrue(Path(self.calls[-1][1]["cwd"]).samefile(data))

    def test_cli_run_reuses_a_matching_build_without_compiler_tools(self):
        self.invoke()
        before = (self.launcher.read_bytes(), self.library.read_bytes())
        self.calls.clear()
        self.run_exit = 17
        self.assertEqual(self.invoke(argv=[str(self.guest), "--output-dir", str(self.output),
                                          "--run", "--", "argument"], tools_available=False), 17)
        self.assertEqual(len(self.calls), 1)
        self.assertTrue(Path(self.calls[0][0][0]).samefile(self.launcher))
        self.assertEqual(self.calls[0][0][1:], ["argument"])
        self.assertEqual(before, (self.launcher.read_bytes(), self.library.read_bytes()))

    def test_cli_run_reuses_older_manifests_without_launcher_hash(self):
        self.invoke()
        record = json.loads(self.manifest.read_text())
        del record["launcher_sha256"]
        self.manifest.write_text(json.dumps(record))
        self.calls.clear()
        self.assertEqual(self.invoke(argv=[str(self.guest), "--output-dir", str(self.output), "--run"]), 0)
        self.assertEqual(len(self.calls), 1)

    def test_cli_run_does_not_reuse_stale_or_incomplete_outputs(self):
        self.invoke()
        original = {p: p.read_bytes() for p in (self.guest, self.library, self.launcher, self.manifest)}
        for path, replacement in ((self.guest, b"changed input"), (self.library, b"changed system"),
                                  (self.launcher, b"changed launcher"), (self.manifest, b"[]"),
                                  (self.manifest, b"broken json"), (self.manifest, None), (self.library, None)):
            with self.subTest(path=path, replacement=replacement):
                for saved, content in original.items():
                    saved.write_bytes(content)
                if replacement is None:
                    path.unlink()
                else:
                    path.write_bytes(replacement)
                self.calls.clear()
                with contextlib.redirect_stderr(io.StringIO()) as errors:
                    self.assertEqual(self.invoke(argv=[str(self.guest), "--output-dir", str(self.output),
                                                      "--run"]), 1)
                self.assertIn("--force", errors.getvalue())
                self.assertEqual(self.calls, [])

    def test_cli_force_run_rebuilds_launcher_and_reuses_compiled_system(self):
        self.invoke()
        self.calls.clear()
        self.assertEqual(self.invoke(argv=[str(self.guest), "--output-dir", str(self.output),
                                          "--run", "--force"]), 0)
        self.assertEqual(self.calls[0][0][0], "rustc")
        self.assertEqual(len(self.calls), 2)

    def test_other_guests_reuse_compiled_system_without_cargo(self):
        self.invoke()
        other = self.root / "SECOND.COM"
        other.write_bytes(b"\xcd\x20")
        self.calls.clear()
        self.assertEqual(self.invoke(argv=[str(other), "--output-dir", str(self.output)],
                                     cargo_available=False), 0)
        self.assertEqual([argv[0] for argv, _ in self.calls], ["rustc"])

    def test_source_changes_and_replaced_libraries_invalidate_system_cache(self):
        source = self.root / "src" / "core.rs"
        source.parent.mkdir()
        source.write_text("// initial system source\n")
        self.invoke()
        self.assertIsNotNone(self.cached_runtime())
        source.write_text("// changed shared system source\n")
        self.assertIsNone(self.cached_runtime())
        self.calls.clear()
        self.invoke(force=True)
        self.assertEqual([argv[0] for argv, _ in self.calls], ["cargo", "rustc"])
        (self.root / "target" / "release" / build_dine.library_name()).write_bytes(b"replaced")
        self.assertIsNone(self.cached_runtime())

    def cached_runtime(self):
        with mock.patch.object(build_dine, "ROOT", self.root):
            return build_dine.cached_runtime()

    def test_run_checks_system_freshness_and_refreshes_library_without_rewrapping(self):
        source = self.root / "src/core.rs"
        source.parent.mkdir()
        source.write_text("// initial system\n")
        self.invoke()
        source.write_text("// changed system\n")
        self.calls.clear()
        self.assertEqual(self.invoke(argv=[str(self.guest), "--output-dir", str(self.output), "--run"]), 0)
        self.assertEqual(self.calls[0][0][0], "cargo")
        self.assertFalse(any(argv[0] == "rustc" for argv, _ in self.calls))

    def test_compare_uses_same_launcher_config_and_boundary_in_both_modes(self):
        self.invoke()
        self.calls.clear()
        with contextlib.redirect_stdout(io.StringIO()) as output:
            self.assertEqual(self.invoke(argv=[str(self.guest), "--output-dir", str(self.output), "--compare"]), 0)
        self.assertEqual(len(self.calls), 2)
        self.assertEqual(self.calls[0][0], self.calls[1][0])
        self.assertEqual(self.calls[0][1]["env"]["PIL_PRECOMPILED"], "off")
        self.assertEqual(self.calls[1][1]["env"]["PIL_PRECOMPILED"], "on")
        self.assertIn("2.00x", output.getvalue())

    def test_compare_rejects_a_ratio_for_different_work(self):
        self.invoke()
        self.comparison_mismatch = True
        with contextlib.redirect_stdout(io.StringIO()) as output, \
             contextlib.redirect_stderr(io.StringIO()) as errors:
            self.assertEqual(self.invoke(argv=[str(self.guest), "--output-dir", str(self.output), "--compare"]), 1)
        self.assertIn("disagree", errors.getvalue())
        self.assertNotIn("speedup:", output.getvalue())

    def test_cli_settings_and_timing_use_shared_library_entry_points(self):
        self.invoke()
        self.calls.clear()
        config = self.root / "configuration with spaces.conf"
        config.write_text("ansi=true\n")
        self.assertEqual(self.invoke(argv=[str(self.guest), "--output-dir", str(self.output),
                                          "--settings", "--time", "--config", str(config)]), 0)
        self.assertEqual(len(self.calls), 2)
        self.assertEqual(self.calls[0][0][1:], ["--dine-settings"])
        self.assertEqual(self.calls[1][0][1:], ["--dine-time"])
        for _, options in self.calls:
            self.assertTrue(Path(options["env"]["PILDOS_CONFIG"]).samefile(config))

    def test_old_launchers_require_rebuild_for_new_host_options(self):
        self.invoke()
        record = json.loads(self.manifest.read_text())
        del record["settings_editor"]
        del record["execution_timing"]
        self.manifest.write_text(json.dumps(record))
        for flag in ("--settings", "--time"):
            self.calls.clear()
            with self.subTest(flag=flag), contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(self.invoke(argv=[str(self.guest), "--output-dir", str(self.output), flag]), 1)
            self.assertEqual(self.calls, [])

    def invoke_invalid(self, path, data):
        path.write_bytes(data)
        with mock.patch.object(build_dine, "ROOT", self.root):
            return build_dine.build(path, self.output)


if __name__ == "__main__":
    unittest.main()
