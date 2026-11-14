#!/usr/bin/env python3
"""Verify every verbatim fixture file matches exact bytes in test_topographic_browser.cpp."""

from __future__ import annotations

import pathlib
import sys


def snake_to_cpp_anchor(stem_prefix: str) -> str:
    if stem_prefix == "local_download_server":
        return "class LocalDownloadServer"
    parts = stem_prefix.split("_")
    return parts[0] + "".join(p.capitalize() for p in parts[1:]) + "("


def find_cpp_source(crate_dir: pathlib.Path) -> pathlib.Path | None:
    rel = pathlib.Path("target") / "cpp-ref" / "tests" / "test_topographic_browser.cpp"
    for ancestor in [crate_dir, *crate_dir.parents]:
        candidate = ancestor / rel
        if candidate.is_file():
            return candidate
    return None


def main() -> int:
    crate_dir = pathlib.Path(__file__).resolve().parent.parent
    fixtures_dir = crate_dir / "fixtures"
    cpp_path = find_cpp_source(crate_dir)
    if cpp_path is None:
        print("ERROR: target/cpp-ref/tests/test_topographic_browser.cpp not found", file=sys.stderr)
        return 1

    if not fixtures_dir.is_dir():
        print(f"ERROR: fixtures directory missing: {fixtures_dir}", file=sys.stderr)
        return 1

    # C++ translation phase 1 maps physical CRLF source checkouts on Windows to LF inside
    # raw string literals; fixture files are stored strictly with LF line endings.
    cpp_bytes = cpp_path.read_bytes().replace(b"\r\n", b"\n")
    fixture_files = sorted(
        p for p in fixtures_dir.iterdir() if p.is_file() and not p.name.startswith(".")
    )
    if not fixture_files:
        print("ERROR: no fixture files found", file=sys.stderr)
        return 1

    missing = 0
    for path in fixture_files:
        data = path.read_bytes()
        if b"\r" in data:
            print(f"FAIL {path.name}: contains CR byte (expected LF only)", file=sys.stderr)
            missing += 1
            continue

        prefix = path.stem.split("__", 1)[0]
        anchor = snake_to_cpp_anchor(prefix).encode("utf-8")
        anchor_pos = cpp_bytes.find(anchor)
        search_start = anchor_pos if anchor_pos != -1 else 0

        pos = cpp_bytes.find(data, search_start)
        if pos == -1 and search_start > 0:
            pos = cpp_bytes.find(data)

        if pos == -1:
            print(f"FAIL {path.name}: exact bytes not found in {cpp_path.name}", file=sys.stderr)
            missing += 1
            continue

        start_line = cpp_bytes.count(b"\n", 0, pos) + 1
        print(f"{path.name}: test_topographic_browser.cpp:{start_line} ({len(data)} bytes)")

    return 1 if missing else 0


if __name__ == "__main__":
    sys.exit(main())
