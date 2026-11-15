#!/usr/bin/env python3
"""Reject personal home/checkout paths in repository text, including docs.

Scan tracked and nonignored new files, including HTML and JSON. Local ignored
configuration, user data and build output are not publication inputs. Standard
OS paths and synthetic path-validation fixtures are not personal locations.
Review other host-path examples for relative/configurable paths as well.
"""
import argparse
import hashlib
import html
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
URL = re.compile(r"https?://[^\s<>\"']+", re.I)
RULES = [
    ("personal Windows directory", re.compile(
        r"(?i)\b[a-z]:[\\/]+(?:users|documents and settings)[\\/]+")),
    ("personal Unix directory", re.compile(
        r"(?i)(?<![\w.])/(?:home|users|root)/[^\s<>\"'`]+")),
    ("host checkout directory", re.compile(
        r"(?i)(?:(?<![\w.])/(?:mnt/(?:[a-z]/(?:users|home)|project-files|data)|workspace|workspaces)/"
        r"|\b[a-z]:[\\/]+(?:workspace|workspaces|repos|projects)[\\/]+)")),
    ("private agent checkout", re.compile(
        r"(?i)(?:\b[a-z]:[\\/]+|(?<![\w.])/(?!/))[^\s<>\"'`]*"
        r"[\\/]\.(?:codex|agents)[\\/]+(?:worktrees|skills|plugins)[\\/]+")),
    ("network home directory", re.compile(
        r"(?i)\\\\[^\\\s]+\\(?:users|homes)[\\/]+")),
]


def findings(text):
    for number, line in enumerate(text.splitlines(), 1):
        line = URL.sub("", html.unescape(line))
        for label, pattern in RULES:
            if pattern.search(line):
                yield number, label


def self_test():
    slash = "/"
    back = chr(92)
    bad = [
        "C:" + back + "Users" + back + "person" + back + "checkout",
        "D:" + back * 2 + "Users" + back * 2 + "person",  # JSON/Rust escaping
        slash.join(["", "home", "person", "checkout"]),
        slash.join(["", "Users", "person", "checkout"]),
        slash.join(["", "mnt", "c", "Users", "person", "checkout"]),
        slash.join(["", "mnt", "project-files", "checkout"]),
        slash.join(["", "workspace", "checkout"]),
        back * 2 + "server" + back + "Users" + back + "person",
        slash.join(["", "somewhere", ".codex", "worktrees", "checkout"]),
    ]
    for sample in bad:
        assert list(findings(sample)), "missed a personal/checkout path"
    assert list(findings(bad[0].replace(back, "&#92;"))), "missed an HTML-encoded path"
    for sample in ["driveD", "../images/disk.img", "${HOME}/data", "$env:USERPROFILE",
                   "C:" + back + "DOSDATA", "/usr/bin/python3", "docs/home/example",
                   "./.codex/skills/example", "../home/example",
                   "https://example.invalid" + slash.join(["", "home", "person"])]:
        assert not list(findings(sample)), "rejected a portable or standard path"
    assert list(findings("first\n" + bad[0]))[0][0] == 2
    print("check_portable_paths self-test passed")


def git_lines(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT).decode("utf-8").split("\0")


def stamp_path():
    git_dir = subprocess.check_output(["git", "rev-parse", "--git-dir"], cwd=ROOT).decode("utf-8").strip()
    path = Path(git_dir)
    return path if path.is_absolute() else ROOT / path


def script_digest():
    return hashlib.sha256(Path(__file__).read_bytes()).hexdigest()


def files_to_check(full=False):
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT).decode("utf-8").strip()
    digest = script_digest()
    stamp_file = stamp_path() / "portable_paths_checked"
    if not full and stamp_file.is_file():
        try:
            stamp = json.loads(stamp_file.read_text(encoding="utf-8"))
        except (OSError, ValueError):
            stamp = None
        if isinstance(stamp, dict) and stamp.get("script_sha256") == digest:
            base = stamp.get("head", "")
            valid_base = bool(base) and subprocess.run(
                ["git", "cat-file", "-e", f"{base}^{{commit}}"],
                cwd=ROOT, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            ).returncode == 0
            if valid_base:
                names = set()
                if base != head:
                    names.update(git_lines("diff", "--name-only", "-z", base, head))
                names.update(git_lines("diff", "--name-only", "-z", "HEAD"))
                names.update(git_lines("ls-files", "-z", "--others", "--exclude-standard"))
                return sorted(filter(None, names)), "incremental", head, digest, stamp_file
    names = git_lines("ls-files", "-z", "--cached", "--others", "--exclude-standard")
    return sorted(set(filter(None, names))), "full", head, digest, stamp_file


def check(full=False):
    names, mode, head, digest, stamp_file = files_to_check(full=full)
    failures = []
    checked = 0
    for name in names:
        path = ROOT / name
        if not path.is_file() or path.is_symlink():
            continue
        data = path.read_bytes()
        try:
            if data.startswith((b"\xff\xfe", b"\xfe\xff")):
                text = data.decode("utf-16")
            elif b"\0" in data:
                continue
            else:
                text = data.decode("utf-8-sig")
        except UnicodeError:
            continue
        checked += 1
        failures.extend(f"{name}:{line}: {label}" for line, label in findings(text))
    if failures:
        print("\n".join(failures))
        raise SystemExit("Replace personal/checkout paths with relative paths or runtime configuration.")
    stamp_file.write_text(json.dumps({"head": head, "script_sha256": digest}) + "\n", encoding="utf-8")
    print(f"No machine-specific home/checkout paths in {checked} {mode} project text files (including documentation).")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--full", action="store_true", help="Scan all tracked and untracked non-ignored files.")
    args = parser.parse_args()
    if args.self_test:
        self_test()
    check(full=args.full)
