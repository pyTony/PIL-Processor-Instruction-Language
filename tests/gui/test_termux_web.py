"""HTTP and low-memory build regressions for the no-X11 Termux frontend."""
import http.client
import importlib.util
import json
from pathlib import Path
import threading
import unittest
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
spec = importlib.util.spec_from_file_location("termux_web", ROOT / "tools/termux_web.py")
web = importlib.util.module_from_spec(spec)
spec.loader.exec_module(web)


class FakeSession:
    def __init__(self):
        self.commands = []

    def send(self, command):
        self.commands.append(command)
        if command.startswith("run "):
            return {"ok": True, "output": "hello", "status": "running"}
        return {
            "ok": True, "status": "running", "screen": "2007" * 2000,
            "columns": 80, "cursor": [0, 0], "steps": 123,
        }


class TermuxWebTests(unittest.TestCase):
    def setUp(self):
        self.session = FakeSession()
        self.backend = web.WebBackend(self.session)
        self.server = web.create_server(self.backend, 0)
        self.assertEqual(self.server.server_address[0], "127.0.0.1")
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.addCleanup(self.thread.join, 2)
        self.addCleanup(self.server.server_close)
        self.addCleanup(self.server.shutdown)

    def request(self, method, path, data=None, headers=None):
        headers = headers or {}
        if data is not None:
            headers.setdefault("Content-Type", "application/json")
            data = json.dumps(data)
        client = http.client.HTTPConnection("127.0.0.1", self.server.server_port, timeout=4)
        try:
            client.request(method, path, body=data, headers=headers)
            reply = client.getresponse()
            return reply.status, reply.read()
        finally:
            client.close()

    def test_abandoned_vga_response_does_not_retry_or_log_broken_pipe(self):
        # Test the HTTP response boundary independently of the DOS engine.
        # Both a failed header flush and a failed body write are common when
        # Chrome switches tabs while a graphics snapshot is in flight.
        handler_type = web.make_handler(self.backend)
        for fail_headers in (False, True):
            handler = object.__new__(handler_type)
            handler.close_connection = False
            handler.send_response = lambda *_args: None
            handler.send_header = lambda *_args: None

            def end_headers():
                if fail_headers:
                    raise BrokenPipeError(32, "broken pipe")
            handler.end_headers = end_headers

            class AbandonedStream:
                def write(self, data):
                    raise BrokenPipeError(32, "broken pipe")
            handler.wfile = AbandonedStream()
            handler.send_json(200, {"graphics_planar": "00" * 32})
            self.assertTrue(handler.close_connection)

    def test_index_and_meta_do_not_require_x11(self):
        code, body = self.request("GET", "/")
        self.assertEqual(code, 200)
        self.assertIn(b"<canvas", body)
        self.assertIn(b"graphics_planar", body)
        self.assertIn(b"/api/key", body)
        code, body = self.request("GET", "/api/meta")
        meta = json.loads(body)
        self.assertEqual(code, 200)
        self.assertEqual(len(meta["glyphs"]), 256)
        self.assertEqual(len(meta["colors"]), 16)
        self.assertNotIn("snapshot", self.session.commands)

    def test_direct_canvas_typing_has_hidden_android_ime_and_dos_enter(self):
        html = (ROOT / "tools/termux_web.html").read_text(encoding="utf-8")
        self.assertIn('id="keyboard"', html)
        self.assertIn('enterkeyhint="enter"', html)
        self.assertIn('id="type" type="text"', html)
        self.assertIn('#type { position:absolute;', html)
        self.assertIn('opacity:0; pointer-events:none;', html)
        self.assertIn('field.addEventListener("input",()=>flushInput())', html)
        self.assertIn('field.addEventListener("compositionend"', html)
        self.assertIn('pendingText=pendingText.catch(()=>{}).then(()=>api("/api/type",{text}))', html)
        self.assertIn('canvas.focus()', html)
        self.assertIn('if(e.pointerType==="touch")field.focus()', html)
        self.assertIn('e.key==="Enter"||e.key==="Backspace"', html)
        self.assertNotIn('id="typeform"', html)
        self.assertNotIn('id="typeonly"', html)

    def test_mount_and_ansi_controls_call_existing_dos_protocol(self):
        import tempfile
        with tempfile.TemporaryDirectory() as folder:
            code, body = self.request("POST", "/api/media",
                                      {"action": "mount", "drive": "d", "path": folder})
            self.assertEqual(code, 200)
            self.assertEqual(self.session.commands[-1], "mount D " + folder)
            code, body = self.request("POST", "/api/media",
                                      {"action": "unmount", "drive": "D"})
            self.assertEqual(code, 200)
            self.assertEqual(self.session.commands[-1], "unmount D")
        code, body = self.request("POST", "/api/ansi", {})
        self.assertEqual(code, 200)
        self.assertEqual(self.session.commands[-1], "ansi on")

    def test_media_rejects_invalid_drives_sources_and_command_injection(self):
        import tempfile
        with tempfile.TemporaryDirectory() as folder:
            for drive in ["F", "", "A B", None]:
                self.assertEqual(self.request("POST", "/api/media",
                    {"action": "mount", "drive": drive, "path": folder})[0], 400)
            for value in ["", "missing-folder-for-pildos-test", folder + "\\nquit"]:
                self.assertEqual(self.request("POST", "/api/media",
                    {"action": "mount", "drive": "D", "path": value})[0], 400)
            self.assertEqual(self.request("POST", "/api/media",
                {"action": "erase", "drive": "D", "path": folder})[0], 400)
        self.assertFalse(any(cmd.startswith("mount ") for cmd in self.session.commands))

    def test_media_controls_render_in_web_page(self):
        code, body = self.request("GET", "/")
        self.assertEqual(code, 200)
        for marker in [b'/api/media', b'/api/ansi', b'id="media-drive"',
                       b'id="media-path"', b'id="ansi-on"', b'id="unmount"']:
            self.assertIn(marker, body)

    def test_android_keyboard_groups_and_screen_controls(self):
        html = (ROOT / "tools/termux_web.html").read_text(encoding="utf-8")
        self.assertIn('class="arrowpad"', html)
        self.assertIn('class="function-keys"', html)
        self.assertIn('id="fullscreen"', html)
        self.assertIn('id="zoomin"', html)
        self.assertIn('id="zoomout"', html)
        self.assertIn('id="zoomfit"', html)
        self.assertIn('class="viewer-keys"', html)
        self.assertIn('id="type" type="text"', html)
        self.assertIn('data-ascii="13" data-scan="28"', html)

    def test_orientation_and_keyboard_viewport_changes_recalculate_fit(self):
        html = (ROOT / "tools/termux_web.html").read_text(encoding="utf-8")
        self.assertIn('window.addEventListener("orientationchange"', html)
        self.assertIn('window.visualViewport.addEventListener("resize",scheduleFit)', html)
        self.assertIn('window.addEventListener("resize",scheduleFit)', html)
        self.assertIn('new ResizeObserver(scheduleFit)', html)
        self.assertIn('height:100dvh', html)
        self.assertIn('env(safe-area-inset-top)', html)
        self.assertIn('Math.min(availableWidth,availableHeight*ratio)', html)
        self.assertIn('requestAnimationFrame', html)

    def test_ctrl_break_feedback_and_text_view_controls_exist(self):
        html = (ROOT / "tools/termux_web.html").read_text(encoding="utf-8")
        self.assertIn('id="interrupt-status"', html)
        self.assertIn('id="view"', html)
        self.assertIn("Ctrl+Break refused:", html)
        self.assertIn("Ctrl+Break delivered.", html)
        self.assertIn("if(forceTextView&&reply.screen)textFrame(reply)", html)
        code, data = self.request("POST", "/api/break", {})
        self.assertEqual(code, 200)
        self.assertTrue(json.loads(data)["ok"])
        self.assertEqual(self.session.commands[-1], "break")

    def test_input_waits_for_bios_buffer_space_instead_of_losing_enter(self):
        original_send = self.session.send
        blocked = [True]

        def congested_send(command):
            if command.startswith("key ") and blocked[0]:
                self.session.commands.append(command)
                return {"ok": False, "text": "BIOS keyboard buffer full"}
            return original_send(command)

        self.session.send = congested_send
        code, response = self.request("POST", "/api/type", {"text": "DIR\n"})
        self.assertEqual(code, 200)
        self.assertEqual(json.loads(response)["queued"], 4)
        self.assertEqual(len(self.backend.pending_keys), 4)
        blocked[0] = False
        code, _ = self.request("POST", "/api/run", {"budget": 1000})
        self.assertEqual(code, 200)
        self.assertEqual(list(self.backend.pending_keys), [])
        self.assertIn("key 13 28", self.session.commands)

    def test_cpu_only_run_skips_large_video_snapshot(self):
        code, body = self.request("POST", "/api/run",
                                  {"budget": 50000, "frame": False})
        state = json.loads(body)
        self.assertEqual(code, 200)
        self.assertTrue(state["frame_pending"])
        self.assertEqual(state["output"], "hello")
        self.assertEqual(self.session.commands, ["run 50000 buffered"])

    def test_frame_option_must_be_boolean(self):
        code, _ = self.request("POST", "/api/run",
                               {"budget": 1000, "frame": "false"})
        self.assertEqual(code, 400)
        self.assertEqual(self.session.commands, [])

    def test_display_refresh_is_capped_without_fixed_active_cpu_delay(self):
        html = (ROOT / "tools/termux_web.html").read_text(encoding="utf-8")
        self.assertIn("const MIN_FRAME_MS=1000/60", html)
        self.assertIn('await api("/api/run",{budget,frame})', html)
        self.assertNotIn("waits.normal", html)

    def test_run_and_snapshot_reuse_existing_engine_protocol(self):
        code, body = self.request("POST", "/api/run", {"budget": 20000})
        state = json.loads(body)
        self.assertEqual(code, 200)
        self.assertEqual(state["status"], "running")
        self.assertEqual(state["output"], "hello")
        self.assertEqual(self.session.commands, ["run 20000 buffered", "snapshot"])
        code, body = self.request("GET", "/api/state")
        self.assertEqual((code, json.loads(body)["steps"]), (200, 123))

    def test_keys_text_and_mouse_use_guest_bios_protocol(self):
        code, body = self.request("POST", "/api/key", {"ascii": 65, "scan": 30})
        self.assertEqual(code, 200)
        self.assertEqual(self.session.commands[-1], "key 65 30")
        code, body = self.request("POST", "/api/type", {"text": "Hi\n"})
        self.assertEqual(code, 200)
        self.assertEqual(self.session.commands[-3:], ["key 72 35", "key 105 23", "key 13 28"])
        code, body = self.request("POST", "/api/mouse", {"x": 320, "y": 100, "buttons": 1})
        self.assertEqual(code, 200)
        self.assertEqual(self.session.commands[-1], "mouse 320 100 1")

    def test_rejects_invalid_input_and_foreign_origins(self):
        self.assertEqual(self.request("POST", "/api/run", {"budget": 1000001})[0], 400)
        self.assertEqual(self.request("POST", "/api/run", {"budget": True})[0], 400)
        self.assertEqual(self.request("POST", "/api/key", {"ascii": -1, "scan": 0})[0], 400)
        self.assertEqual(self.request("POST", "/api/type", {"text": "x" * 257})[0], 400)
        self.assertEqual(self.request("POST", "/api/mouse", {"x": 0, "y": 0, "buttons": 8})[0], 400)
        code, body = self.request("POST", "/api/run", {"budget": 4},
                                  {"Origin": "https://unrelated.example"})
        self.assertEqual(code, 403)
        self.assertEqual(self.request("POST", "/api/run", {"budget": 4},
                                      {"Content-Type": "text/plain"})[0], 415)
        self.assertEqual(self.request("GET", "/../file")[0], 404)
        self.assertEqual(self.session.commands, [])

    def test_build_script_serial_and_no_cargo_run(self):
        script = (ROOT / "tools/build_termux.sh").read_text(encoding="utf-8")
        self.assertIn("CARGO_BUILD_JOBS=1", script)
        self.assertIn("CARGO_INCREMENTAL=0", script)
        self.assertIn("CARGO_PROFILE_DEV_DEBUG=0", script)
        self.assertIn("cargo build --locked --bin pil -j 1", script)
        self.assertNotIn("\ncargo run ", script)


if __name__ == "__main__":
    unittest.main()
