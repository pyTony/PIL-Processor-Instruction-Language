#!/usr/bin/env python3
"""Loopback-only, stdlib web frontend for Dine on Android/Termux (no X11/Tk).

The browser paints DOS screen snapshots. All guest execution, BIOS, DOS,
keyboard and VGA semantics remain in the existing pil --dos-session engine.
"""
import argparse
from collections import deque
from datetime import datetime
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import shutil
import subprocess
import threading
from urllib.parse import urlsplit

from dos_terminal import GLYPHS, PALETTE, Session, configured_startup, find_dos_engine, paste_keys

HOST = "127.0.0.1"
DEFAULT_PORT = 8765
MAX_REQUEST = 4096
MAX_RUN = 100000
MAX_QUEUED_KEYS = 512
HTML = Path(__file__).with_suffix(".html")


class WebBackend:
    """Serialize all accesses to the single guest session from HTTP threads."""

    def __init__(self, session):
        self.session = session
        self.lock = threading.Lock()
        # BIOS has a small keyboard ring. Keep browser keys on the host until
        # the guest has actually accepted them; a burst of pasted text must
        # never silently lose its final Enter or Backspace.
        self.pending_keys = deque()

    def _drain_keys(self):
        while self.pending_keys:
            ascii_code, scan = self.pending_keys[0]
            reply = self.session.send("key %d %d" % (ascii_code, scan))
            if not reply.get("ok", False):
                if "buffer full" in str(reply.get("text", "")).lower():
                    break
                if "buffer full" in str(reply.get("error", "")).lower():
                    break
                raise RuntimeError("DOS key rejected: %s" % reply)
            self.pending_keys.popleft()

    def _queue_keys(self, keys):
        if len(self.pending_keys) + len(keys) > MAX_QUEUED_KEYS:
            raise ValueError("DOS input queue full; wait for the program to read keys")
        self.pending_keys.extend(keys)
        self._drain_keys()
        return {"ok": True, "queued": len(self.pending_keys)}

    def snapshot(self):
        with self.lock:
            return self.session.send("snapshot")

    def run(self, budget, frame=True):
        if not isinstance(budget, int) or isinstance(budget, bool) or not 1 <= budget <= MAX_RUN:
            raise ValueError("run budget must be 1..100000")
        if not isinstance(frame, bool):
            raise ValueError("frame must be a boolean")
        with self.lock:
            self._drain_keys()
            previous = self.session.send("run %d buffered" % budget)
            self._drain_keys()
            # The CPU may run many slices between displays; avoid serializing
            # a full VGA framebuffer unless the browser requests a refresh.
            if not frame:
                # No framebuffer is serialized, but tell callers a fresh
                # display snapshot is still outstanding.
                previous["frame_pending"] = True
                return previous
            snapshot = self.session.send("snapshot")
            if previous.get("output"):
                snapshot["output"] = previous["output"]
            return snapshot

    def key(self, ascii_code, scan):
        if (not isinstance(ascii_code, int) or isinstance(ascii_code, bool)
                or not 0 <= ascii_code <= 255 or not isinstance(scan, int)
                or isinstance(scan, bool) or not 0 <= scan <= 255):
            raise ValueError("key requires BIOS ASCII and scan-code bytes")
        with self.lock:
            return self._queue_keys([(ascii_code, scan)])

    def mouse(self, x, y, buttons):
        if not all(isinstance(v, int) and not isinstance(v, bool) for v in (x, y, buttons)):
            raise ValueError("mouse coordinates must be integers")
        if not (0 <= x <= 65535 and 0 <= y <= 65535 and 0 <= buttons <= 7):
            raise ValueError("mouse coordinates/buttons out of range")
        with self.lock:
            return self.session.send("mouse %d %d %d" % (x, y, buttons))

    def type_text(self, message):
        if not isinstance(message, str) or len(message) > 256:
            raise ValueError("text must have at most 256 characters")
        keys = paste_keys(message)
        with self.lock:
            return self._queue_keys(keys)

    def video_watch(self, enabled):
        if not isinstance(enabled, bool):
            raise ValueError("video diagnostics enabled must be a boolean")
        with self.lock:
            reply = self.session.send("video-watch " + ("on" if enabled else "off"))
            if not reply.get("ok", False):
                raise RuntimeError(reply.get("text") or "video diagnostics request rejected")
            return reply

    def ansi_on(self):
        # DOS has an install command but no uninstall command. Enabling it
        # affects the existing guest session without restarting the program.
        with self.lock:
            return self.session.send("ansi on")

    def media(self, action, drive, path=None):
        if action not in ("mount", "unmount"):
            raise ValueError("media action must be mount or unmount")
        if not isinstance(drive, str) or drive.upper() not in ("A", "B", "C", "D", "E"):
            raise ValueError("drive must be A through E")
        letter = drive.upper()
        if action == "mount":
            if not isinstance(path, str) or not path.strip() or len(path) > 768:
                raise ValueError("mount path must be 1..768 characters")
            # The engine protocol is line-based: never allow a browser path
            # to inject a second command (or hidden control characters).
            if any(ord(char) < 32 or ord(char) == 127 for char in path):
                raise ValueError("mount path contains a control character")
            resolved = Path(path.strip()).expanduser()
            if not resolved.is_absolute():
                resolved = Path.cwd() / resolved
            if not (resolved.is_dir() or (resolved.is_file()
                    and resolved.suffix.lower() in (".img", ".ima"))):
                raise ValueError("mount source must be an existing directory or .img/.ima file on Termux")
            command = "mount %s %s" % (letter, resolved)
        else:
            command = "unmount %s" % letter
        with self.lock:
            return self.session.send(command)

    def stop(self):
        with self.lock:
            return self.session.send("break")


def make_handler(backend, html=HTML):
    """A fresh handler class per server, so isolated unit tests need no PIL."""

    class Handler(BaseHTTPRequestHandler):
        server_version = "PILdosTermuxWeb/1"
        protocol_version = "HTTP/1.1"

        def log_message(self, format_string, *args):
            # Avoid logging DOS input or process-state JSON to the console.
            if self.path not in ("/api/run", "/api/state"):
                super().log_message(format_string, *args)

        def send_bytes(self, status, body, content_type):
            self.send_response(status)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Cache-Control", "no-store")
            self.send_header("X-Content-Type-Options", "nosniff")
            self.send_header("Referrer-Policy", "no-referrer")
            self.send_header("Cross-Origin-Resource-Policy", "same-origin")
            # Browsers can abandon a large VGA snapshot when a tab is hidden
            # or refreshed. The client has gone: do not try to send a second
            # (503) response on the same broken connection.
            try:
                self.end_headers()
                self.wfile.write(body)
            except (BrokenPipeError, ConnectionResetError, ConnectionAbortedError):
                self.close_connection = True

        def send_json(self, status, value):
            self.send_bytes(status, json.dumps(value, ensure_ascii=False).encode("utf-8"),
                            "application/json; charset=utf-8")

        def do_GET(self):
            path = urlsplit(self.path).path
            if path == "/":
                self.send_bytes(200, html.read_bytes(), "text/html; charset=utf-8")
            elif path == "/api/meta":
                self.send_json(200, {"glyphs": GLYPHS, "colors": PALETTE})
            elif path == "/api/state":
                try:
                    self.send_json(200, backend.snapshot())
                except (OSError, RuntimeError, ValueError) as error:
                    self.send_json(503, {"error": str(error)})
            else:
                self.send_json(404, {"error": "not found"})

        def do_POST(self):
            path = urlsplit(self.path).path
            # No wildcard CORS, no proxy or remote listener. Block forms and
            # cross-origin fetches from other pages on the Android device.
            origin = self.headers.get("Origin")
            expected = "http://127.0.0.1:%d" % self.server.server_port
            if origin and origin != expected:
                return self.send_json(403, {"error": "cross-origin requests forbidden"})
            if self.headers.get("Content-Type", "").split(";")[0].strip() != "application/json":
                return self.send_json(415, {"error": "expected application/json"})
            try:
                length = int(self.headers.get("Content-Length", "-1"))
                if not 0 <= length <= MAX_REQUEST:
                    return self.send_json(413, {"error": "invalid request size"})
                body = json.loads(self.rfile.read(length))
                if not isinstance(body, dict):
                    raise ValueError("expected a JSON object")
                if path == "/api/run":
                    reply = backend.run(body.get("budget"), body.get("frame", True))
                elif path == "/api/key":
                    reply = backend.key(body.get("ascii"), body.get("scan"))
                elif path == "/api/type":
                    reply = backend.type_text(body.get("text"))
                elif path == "/api/mouse":
                    reply = backend.mouse(body.get("x"), body.get("y"), body.get("buttons"))
                elif path == "/api/break":
                    reply = backend.stop()
                elif path == "/api/ansi":
                    reply = backend.ansi_on()
                elif path == "/api/video-watch":
                    reply = backend.video_watch(body.get("enabled"))
                elif path == "/api/media":
                    reply = backend.media(body.get("action"), body.get("drive"), body.get("path"))
                else:
                    return self.send_json(404, {"error": "not found"})
                self.send_json(200, reply)
            except (ValueError, TypeError) as error:
                self.send_json(400, {"error": str(error)})
            except (OSError, RuntimeError) as error:
                self.send_json(503, {"error": str(error)})

    return Handler


def create_server(backend, port=DEFAULT_PORT, html=HTML):
    if not isinstance(port, int) or not 0 <= port < 65536:
        raise ValueError("port must be 0..65535")
    return ThreadingHTTPServer((HOST, port), make_handler(backend, html))


def configure_host_clock(session, offset=None):
    """Enable the DOS host clock, matching the native desktop frontend."""
    if offset is None:
        offset = datetime.now().astimezone().utcoffset()
        offset = int(offset.total_seconds()) if offset is not None else 0
    reply = session.send("clock %d" % offset)
    if not reply.get("ok", False):
        raise RuntimeError("DOS clock initialization failed: %s" % reply)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("program", nargs="?", help="DOS COM/EXE or IMAGE.img::PATH (default: pildos.conf)")
    parser.add_argument("--engine", help="existing pil binary; does not compile Rust")
    parser.add_argument("--config", help="PILdos configuration path")
    parser.add_argument("--mount", help="additional host directory as DOS drive")
    parser.add_argument("--args", help="guest DOS command tail")
    parser.add_argument("--fpu", choices=("none", "80387"), default="none")
    parser.add_argument("--ansi", action="store_true")
    parser.add_argument("--mouse", action="store_true", help="opt in to DOS INT 33h mouse")
    parser.add_argument("--port", type=int, default=DEFAULT_PORT)
    parser.add_argument("--open", action="store_true", help="open URL with termux-open-url")
    options = parser.parse_args(argv)
    engine = find_dos_engine(options.engine)
    program = options.program
    command_tail = options.args
    if not program:
        program, default_args = configured_startup(engine, options.config)
        if command_tail is None:
            command_tail = default_args
    session = Session(engine, program, mount=options.mount, args=command_tail,
                      config=options.config, fpu=options.fpu, mouse=options.mouse,
                      ansi=options.ansi, stop_log="PILdos-stops.jsonl",
                      browser_video=True)
    try:
        configure_host_clock(session)
        backend = WebBackend(session)
        with create_server(backend, options.port) as server:
            address = "http://127.0.0.1:%d/" % server.server_port
            print("PILdos Android GUI:", address, flush=True)
            print("Open the address in Android Chrome/Firefox. Ctrl+C stops the server.", flush=True)
            if options.open and shutil.which("termux-open-url"):
                subprocess.run(["termux-open-url", address], check=False)
            server.serve_forever(poll_interval=0.2)
    except KeyboardInterrupt:
        pass
    finally:
        session.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
