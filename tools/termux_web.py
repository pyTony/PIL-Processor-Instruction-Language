#!/usr/bin/env python3
"""Loopback-only, stdlib web frontend for Dine on Android/Termux (no X11/Tk).

The browser paints DOS screen snapshots. All guest execution, BIOS, DOS,
keyboard and VGA semantics remain in the existing pil --dos-session engine.
"""
import argparse
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
HTML = Path(__file__).with_suffix(".html")


class WebBackend:
    """Serialize all accesses to the single guest session from HTTP threads."""

    def __init__(self, session):
        self.session = session
        self.lock = threading.Lock()

    def snapshot(self):
        with self.lock:
            return self.session.send("snapshot")

    def run(self, budget):
        if not isinstance(budget, int) or isinstance(budget, bool) or not 1 <= budget <= MAX_RUN:
            raise ValueError("run budget must be 1..100000")
        with self.lock:
            previous = self.session.send("run %d buffered" % budget)
            frame = self.session.send("snapshot")
            if previous.get("output"):
                frame["output"] = previous["output"]
            return frame

    def key(self, ascii_code, scan):
        if (not isinstance(ascii_code, int) or isinstance(ascii_code, bool)
                or not 0 <= ascii_code <= 255 or not isinstance(scan, int)
                or isinstance(scan, bool) or not 0 <= scan <= 255):
            raise ValueError("key requires BIOS ASCII and scan-code bytes")
        with self.lock:
            return self.session.send("key %d %d" % (ascii_code, scan))

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
            reply = None
            for ascii_code, scan in keys:
                reply = self.session.send("key %d %d" % (ascii_code, scan))
                if not reply.get("ok", False):
                    break
            return reply if reply is not None else self.session.send("snapshot")

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
            self.end_headers()
            self.wfile.write(body)

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
                    reply = backend.run(body.get("budget"))
                elif path == "/api/key":
                    reply = backend.key(body.get("ascii"), body.get("scan"))
                elif path == "/api/type":
                    reply = backend.type_text(body.get("text"))
                elif path == "/api/mouse":
                    reply = backend.mouse(body.get("x"), body.get("y"), body.get("buttons"))
                elif path == "/api/break":
                    reply = backend.stop()
                else:
                    return self.send_json(404, {"error": "not found"})
                self.send_json(200, reply)
            except (ValueError, TypeError) as error:
                self.send_json(400, {"error": str(error)})
            except (OSError, RuntimeError, BrokenPipeError) as error:
                self.send_json(503, {"error": str(error)})

    return Handler


def create_server(backend, port=DEFAULT_PORT, html=HTML):
    if not isinstance(port, int) or not 0 <= port < 65536:
        raise ValueError("port must be 0..65535")
    return ThreadingHTTPServer((HOST, port), make_handler(backend, html))


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
                      ansi=options.ansi, stop_log="PILdos-stops.jsonl")
    try:
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
