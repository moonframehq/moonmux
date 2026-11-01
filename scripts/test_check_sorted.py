"""Tests for scripts/check-sorted.py."""

import importlib.util
import sys
import unittest
from pathlib import Path

# Loading the hyphenated script as a module would leave a __pycache__ beside it.
sys.dont_write_bytecode = True
SPEC = importlib.util.spec_from_file_location(
    "check_sorted", Path(__file__).with_name("check-sorted.py")
)
assert SPEC and SPEC.loader
check_sorted = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(check_sorted)


def region(*lines: str) -> str:
    return "\n".join(["before", "// keep-sorted start", *lines, "// keep-sorted end", "after", ""])


class CheckSortedTests(unittest.TestCase):
    def test_sorted_region_is_unchanged(self) -> None:
        text = region('    "a.svg",', '    "B.svg",', '    "c.svg",')
        self.assertEqual(check_sorted.process(text), (text, []))

    def test_unsorted_region_is_reported_at_its_first_line_and_fixed(self) -> None:
        fixed, unsorted = check_sorted.process(region("    b,", "    a,"))
        self.assertEqual(unsorted, [2])
        self.assertEqual(fixed, region("    a,", "    b,"))

    def test_doc_comments_and_attributes_move_with_their_entry(self) -> None:
        text = region(
            "    /// Zed.",
            "    #[cfg(test)]",
            "    Zed {",
            "        field: u8,",
            "    },",
            "    /// Alpha.",
            "    Alpha,",
        )
        fixed, _ = check_sorted.process(text)
        self.assertEqual(
            fixed,
            region(
                "    /// Alpha.",
                "    Alpha,",
                "    /// Zed.",
                "    #[cfg(test)]",
                "    Zed {",
                "        field: u8,",
                "    },",
            ),
        )

    def test_blank_lines_separate_independently_sorted_blocks(self) -> None:
        text = region("    c,", "    d,", "", "    a,", "    b,")
        self.assertEqual(check_sorted.process(text), (text, []))

    def test_visibility_does_not_affect_order(self) -> None:
        text = region("    pub(crate) alpha: u8,", "    beta: u8,", "    pub(super) gamma: u8,", "    pub zeta: u8,")
        self.assertEqual(check_sorted.process(text), (text, []))

    def test_multi_line_attributes_do_not_set_the_key(self) -> None:
        text = region(
            "    alpha: u8,",
            "    #[cfg_attr(",
            "        not(test),",
            "        expect(dead_code)",
            "    )]",
            "    beta: u8,",
        )
        self.assertEqual(check_sorted.process(text), (text, []))

    def test_hash_comments_move_with_their_entry(self) -> None:
        text = "# keep-sorted start\n# Zed.\nzed = 1\nalpha = 2\n# keep-sorted end\n"
        fixed, unsorted = check_sorted.process(text)
        self.assertEqual(unsorted, [1])
        self.assertEqual(fixed, "# keep-sorted start\nalpha = 2\n# Zed.\nzed = 1\n# keep-sorted end\n")

    def test_markers_outside_comments_are_ignored(self) -> None:
        text = 'MARKER = "// keep-sorted start"\nb\na\n'
        self.assertEqual(check_sorted.process(text), (text, []))

    def test_missing_end_is_an_error(self) -> None:
        with self.assertRaises(check_sorted.RegionError):
            check_sorted.process("// keep-sorted start\n    a,\n")

    def test_trailing_comment_without_entry_is_an_error(self) -> None:
        with self.assertRaises(check_sorted.RegionError):
            check_sorted.process(region("    a,", "    // orphan"))


if __name__ == "__main__":
    unittest.main()
