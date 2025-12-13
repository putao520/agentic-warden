#!/usr/bin/env python3
"""
Generate numbered test output lines.

Default output produces 150 lines, each annotated with its line number.
"""
from __future__ import annotations

import argparse
from typing import List


def generate_test_output(line_count: int = 150) -> List[str]:
    """
    Build a list of test output strings with embedded line numbers.

    Args:
        line_count: Total number of lines to generate. Must be positive.

    Returns:
        List of formatted strings such as "Line 001: Test output line 1".

    Raises:
        ValueError: If line_count is not a positive integer.
    """
    if line_count < 1:
        raise ValueError("line_count must be a positive integer")

    width = max(3, len(str(line_count)))
    return [f"Line {i:0{width}d}: Test output line {i}" for i in range(1, line_count + 1)]


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    """Parse CLI arguments for custom line counts."""
    parser = argparse.ArgumentParser(description="Generate test output with numbered lines.")
    parser.add_argument(
        "-n",
        "--lines",
        type=int,
        default=150,
        help="Number of lines to generate (default: 150).",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    """Entry point for the CLI."""
    args = parse_args(argv)
    for line in generate_test_output(args.lines):
        print(line)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
