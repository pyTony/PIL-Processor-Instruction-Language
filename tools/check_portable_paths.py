#!/usr/bin/env python3
"""Reject personal home/checkout paths in repository text, including docs.

Scan tracked and nonignored new files, including HTML and JSON. Local ignored
configuration, user data and build output are not publication inputs. Standard
OS paths and synthetic path-validation fixtures are not personal locations.
Review other host-path examples for relative/configurable paths as well.
"""
import argparse
import html
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


def check():
    names = subprocess.check_output(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"], cwd=ROOT
    ).decode("utf-8").split("\0")
    failures = []
    checked = 0
    for name in sorted(set(filter(None, names))):
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
    print(f"No machine-specific home/checkout paths in {checked} project text files (including documentation).")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
    check()
