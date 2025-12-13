"""
Unit tests for scripts.generate_test_output.
"""
from __future__ import annotations

import importlib.util
import pathlib
import unittest


def _load_module():
    module_path = pathlib.Path(__file__).resolve().parents[1] / "generate_test_output.py"
    spec = importlib.util.spec_from_file_location("generate_test_output", module_path)
    if spec is None or spec.loader is None:
        raise ImportError(f"Could not load module from {module_path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class GenerateTestOutputTests(unittest.TestCase):
    def setUp(self):
        module = _load_module()
        self.generate_test_output = module.generate_test_output

    def test_default_line_count_is_150(self):
        lines = self.generate_test_output()
        self.assertEqual(len(lines), 150)
        self.assertTrue(lines[0].startswith("Line 001"))
        self.assertIn("Test output line 150", lines[-1])

    def test_custom_line_count_and_padding(self):
        lines = self.generate_test_output(5)
        self.assertEqual(len(lines), 5)
        self.assertEqual(lines[0], "Line 001: Test output line 1")
        self.assertEqual(lines[-1], "Line 005: Test output line 5")

    def test_line_count_above_thousand_expands_width(self):
        lines = self.generate_test_output(1200)
        self.assertTrue(lines[0].startswith("Line 0001"))
        self.assertIn("Line 1200", lines[-1])
        self.assertEqual(len(lines), 1200)

    def test_reject_non_positive_counts(self):
        with self.assertRaises(ValueError):
            self.generate_test_output(0)
        with self.assertRaises(ValueError):
            self.generate_test_output(-5)


if __name__ == "__main__":
    unittest.main()
