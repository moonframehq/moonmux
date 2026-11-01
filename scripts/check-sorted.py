#!/usr/bin/env python3
"""Keep marked lists sorted, so parallel additions land on different lines.

A region runs from a comment line (`//` or `#`) containing `keep-sorted start`
to one containing `keep-sorted end`. Its entries start at the region's own indentation; comment
and attribute lines (`//`, `#`) before an entry belong to it, and deeper
indented or closing lines after it continue it. Blank lines split a region
into blocks that are each sorted on their own. Entries compare by their first
line that is not a comment or an attribute, ignoring case and a leading
`pub` or `pub(...)`, so struct fields sort by name whatever their visibility.

    python3 scripts/check-sorted.py          # report unsorted regions
    python3 scripts/check-sorted.py --fix    # sort them in place
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

START = "keep-sorted start"
END = "keep-sorted end"
PATTERNS = ("*.rs", "*.toml", "*.py", "*.sh")
VISIBILITY = re.compile(r"^pub(\([^)]*\))?\s+")


class RegionError(Exception):
    pass


def is_marker(line: str, marker: str) -> bool:
    return line.lstrip().startswith(("//", "#")) and marker in line


def indent(line: str) -> int:
    return len(line) - len(line.lstrip())


def is_preamble(stripped: str) -> bool:
    # `#` covers Rust attributes and the comments of Python, shell, and TOML.
    return stripped.startswith(("//", "#"))


def is_continuation(stripped: str) -> bool:
    return stripped[:1] in ")]}"


def key(entry: list[str]) -> str:
    """The entry's own line: at its indentation, after any comments and
    attributes, whose continuation lines are skipped with them."""
    base = indent(entry[0])
    for line in entry:
        stripped = line.strip()
        if indent(line) == base and not is_preamble(stripped) and not is_continuation(stripped):
            return VISIBILITY.sub("", stripped).casefold()
    return entry[0].strip().casefold()


def entries(block: list[str]) -> list[list[str]]:
    """Split one blank-line-free block into entries."""
    base = min(indent(line) for line in block)
    result: list[list[str]] = []
    pending: list[str] = []
    for line in block:
        stripped = line.strip()
        starts = indent(line) == base and not is_continuation(stripped)
        if starts and is_preamble(stripped):
            pending.append(line)
        elif starts:
            result.append([*pending, line])
            pending = []
        elif pending:
            pending.append(line)
        elif result:
            result[-1].append(line)
        else:
            raise RegionError("region starts with an indented line")
    if pending:
        raise RegionError("comment or attribute without an entry after it")
    return result


def sort_region(lines: list[str]) -> list[str]:
    """The region's lines with every block sorted."""
    out: list[str] = []
    block: list[str] = []

    def flush() -> None:
        if block:
            for entry in sorted(entries(block), key=key):
                out.extend(entry)
            block.clear()

    for line in lines:
        if line.strip():
            block.append(line)
        else:
            flush()
            out.append(line)
    flush()
    return out


def process(text: str) -> tuple[str, list[int]]:
    """The text with regions sorted, and the start lines of unsorted ones."""
    lines = text.splitlines(keepends=True)
    out: list[str] = []
    unsorted: list[int] = []
    i = 0
    while i < len(lines):
        out.append(lines[i])
        if not is_marker(lines[i], START):
            i += 1
            continue
        start = i + 1
        end = next((j for j in range(start, len(lines)) if is_marker(lines[j], END)), None)
        if end is None:
            raise RegionError(f"line {start}: `{START}` without `{END}`")
        region = lines[start:end]
        try:
            sorted_region = sort_region(region)
        except RegionError as error:
            raise RegionError(f"line {start}: {error}") from error
        if sorted_region != region:
            unsorted.append(start)
        out.extend(sorted_region)
        i = end
    return "".join(out), unsorted


def tracked_files() -> list[Path]:
    listing = subprocess.run(
        ["git", "ls-files", "-z", "--", *PATTERNS],
        check=True,
        capture_output=True,
    ).stdout
    return [Path(name) for name in listing.decode().split("\0") if name]


def main(argv: list[str]) -> int:
    fix = "--fix" in argv[1:]
    failures = 0
    for path in tracked_files():
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8")
        if START not in text:
            continue
        try:
            fixed, unsorted = process(text)
        except RegionError as error:
            print(f"FAIL: {path}: {error}")
            failures += 1
            continue
        if not unsorted:
            continue
        if fix:
            path.write_text(fixed, encoding="utf-8")
            print(f"sorted: {path}")
        else:
            for line in unsorted:
                print(f"FAIL: {path}:{line}: keep-sorted region is not sorted")
            failures += 1
    if failures:
        print("\nRun `just sort` to sort the marked regions.")
        return 1
    print("All keep-sorted regions are sorted.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
