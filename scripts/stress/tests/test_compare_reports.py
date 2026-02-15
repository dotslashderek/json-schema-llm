"""Tests for compare_reports.py — #119 Report Diffing Script."""

import json
import tempfile
from pathlib import Path

import pytest


def _make_report(detailed_results, pass_list=None, fail_list=None):
    """Build a minimal stress-test report dict."""
    return {
        "metadata": {
            "model": "gpt-4o-mini",
            "schema_count": len(detailed_results),
            "timestamp": "2026-01-01T00:00:00Z",
        },
        "pass": pass_list or [],
        "fail": fail_list or [],
        "detailed_results": detailed_results,
    }


def _make_result(file_name, classification):
    """Create a single detailed_result entry."""
    return {
        "file": f"{file_name}.json",
        "classification": classification,
        "verdict": "solid_pass" if "pass" in classification else "solid_fail",
        "attempts": [
            {
                "passed": "pass" in classification,
                "stage": None if "pass" in classification else "openai",
                "reason": None if "pass" in classification else "api_error",
                "error": "",
            }
        ],
    }


def _write_report(tmp_dir, name, detailed_results):
    """Write a report JSON to a temp file and return its path."""
    report = _make_report(detailed_results)
    path = tmp_dir / f"{name}.json"
    path.write_text(json.dumps(report, indent=2))
    return str(path)


def _load_compare_module():
    """Load compare_reports.py as a module."""
    import importlib.util

    script = Path(__file__).parent.parent / "compare_reports.py"
    spec = importlib.util.spec_from_file_location("compare_reports", script)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


class TestCompareReports:
    """#119: Report diffing for regression tracking."""

    def test_identical_reports(self):
        """Two identical reports should show no changes."""
        mod = _load_compare_module()
        results = [
            _make_result("schema_a", "solid_pass"),
            _make_result("schema_b", "solid_fail"),
        ]
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            baseline = _write_report(tmp_path, "baseline", results)
            current = _write_report(tmp_path, "current", results)
            result = mod.compare_reports(
                mod.load_report(baseline), mod.load_report(current)
            )

        assert len(result.new_passes) == 0
        assert len(result.new_failures) == 0
        assert len(result.fixes) == 0
        assert len(result.new_flaky) == 0
        assert len(result.config_drift) == 0
        assert len(result.unchanged) == 2

    def test_new_passes_detected(self):
        """Schema going from solid_fail → solid_pass (not in expected_failures)."""
        mod = _load_compare_module()
        baseline_results = [_make_result("schema_a", "solid_fail")]
        current_results = [_make_result("schema_a", "solid_pass")]
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            baseline = _write_report(tmp_path, "baseline", baseline_results)
            current = _write_report(tmp_path, "current", current_results)
            result = mod.compare_reports(
                mod.load_report(baseline), mod.load_report(current)
            )

        assert "schema_a" in result.new_passes

    def test_new_failures_detected(self):
        """Schema going from solid_pass → solid_fail."""
        mod = _load_compare_module()
        baseline_results = [_make_result("schema_a", "solid_pass")]
        current_results = [_make_result("schema_a", "solid_fail")]
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            baseline = _write_report(tmp_path, "baseline", baseline_results)
            current = _write_report(tmp_path, "current", current_results)
            result = mod.compare_reports(
                mod.load_report(baseline), mod.load_report(current)
            )

        assert "schema_a" in result.new_failures

    def test_flaky_changes_detected(self):
        """solid_pass → flaky_pass categorized as new_flaky."""
        mod = _load_compare_module()
        baseline_results = [_make_result("schema_a", "solid_pass")]
        current_results = [_make_result("schema_a", "flaky_pass")]
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            baseline = _write_report(tmp_path, "baseline", baseline_results)
            current = _write_report(tmp_path, "current", current_results)
            result = mod.compare_reports(
                mod.load_report(baseline), mod.load_report(current)
            )

        assert "schema_a" in result.new_flaky

    def test_fixes_detected(self):
        """expected_fail → solid_pass categorized as fix."""
        mod = _load_compare_module()
        baseline_results = [_make_result("schema_a", "expected_fail")]
        current_results = [_make_result("schema_a", "solid_pass")]
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            baseline = _write_report(tmp_path, "baseline", baseline_results)
            current = _write_report(tmp_path, "current", current_results)
            result = mod.compare_reports(
                mod.load_report(baseline), mod.load_report(current)
            )

        assert "schema_a" in result.fixes

    def test_config_drift_detected(self):
        """unexpected_pass → solid_pass categorized as config_drift."""
        mod = _load_compare_module()
        baseline_results = [_make_result("schema_a", "unexpected_pass")]
        current_results = [_make_result("schema_a", "solid_pass")]
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            baseline = _write_report(tmp_path, "baseline", baseline_results)
            current = _write_report(tmp_path, "current", current_results)
            result = mod.compare_reports(
                mod.load_report(baseline), mod.load_report(current)
            )

        assert "schema_a" in result.config_drift

    def test_schemas_added_removed(self):
        """Different schema sets tracked as baseline_only / current_only."""
        mod = _load_compare_module()
        baseline_results = [
            _make_result("only_in_baseline", "solid_pass"),
            _make_result("common", "solid_pass"),
        ]
        current_results = [
            _make_result("common", "solid_pass"),
            _make_result("only_in_current", "solid_pass"),
        ]
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            baseline = _write_report(tmp_path, "baseline", baseline_results)
            current = _write_report(tmp_path, "current", current_results)
            result = mod.compare_reports(
                mod.load_report(baseline), mod.load_report(current)
            )

        assert "only_in_baseline" in result.baseline_only
        assert "only_in_current" in result.current_only

    def test_pass_rate_calculation(self):
        """Pass rates computed correctly."""
        mod = _load_compare_module()
        baseline_results = [
            _make_result("a", "solid_pass"),
            _make_result("b", "solid_fail"),
        ]
        current_results = [
            _make_result("a", "solid_pass"),
            _make_result("b", "solid_pass"),
        ]
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            baseline = _write_report(tmp_path, "baseline", baseline_results)
            current = _write_report(tmp_path, "current", current_results)
            result = mod.compare_reports(
                mod.load_report(baseline), mod.load_report(current)
            )

        assert result.baseline_pass_rate == pytest.approx(50.0)
        assert result.current_pass_rate == pytest.approx(100.0)

    def test_exit_code_zero_no_regressions(self):
        """No new failures → exit 0."""
        mod = _load_compare_module()
        baseline_results = [_make_result("a", "solid_pass")]
        current_results = [_make_result("a", "solid_pass")]
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            baseline = _write_report(tmp_path, "baseline", baseline_results)
            current = _write_report(tmp_path, "current", current_results)
            exit_code = mod.get_exit_code(
                mod.compare_reports(
                    mod.load_report(baseline), mod.load_report(current)
                ),
                strict=False,
            )
        assert exit_code == 0

    def test_exit_code_one_with_regressions(self):
        """New failures → exit 1."""
        mod = _load_compare_module()
        baseline_results = [_make_result("a", "solid_pass")]
        current_results = [_make_result("a", "solid_fail")]
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            baseline = _write_report(tmp_path, "baseline", baseline_results)
            current = _write_report(tmp_path, "current", current_results)
            exit_code = mod.get_exit_code(
                mod.compare_reports(
                    mod.load_report(baseline), mod.load_report(current)
                ),
                strict=False,
            )
        assert exit_code == 1

    def test_strict_mode_exits_on_flakiness(self):
        """--strict + new flaky → exit 1."""
        mod = _load_compare_module()
        baseline_results = [_make_result("a", "solid_pass")]
        current_results = [_make_result("a", "flaky_pass")]
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            baseline = _write_report(tmp_path, "baseline", baseline_results)
            current = _write_report(tmp_path, "current", current_results)
            exit_code = mod.get_exit_code(
                mod.compare_reports(
                    mod.load_report(baseline), mod.load_report(current)
                ),
                strict=True,
            )
        assert exit_code == 1

    def test_json_output_flag(self):
        """--json produces valid JSON output."""
        mod = _load_compare_module()
        baseline_results = [_make_result("a", "solid_pass")]
        current_results = [_make_result("a", "solid_fail")]
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            baseline = _write_report(tmp_path, "baseline", baseline_results)
            current = _write_report(tmp_path, "current", current_results)
            result = mod.compare_reports(
                mod.load_report(baseline), mod.load_report(current)
            )
            output = mod.format_comparison(result, json_output=True)
        parsed = json.loads(output)
        assert "new_failures" in parsed
        assert isinstance(parsed["new_failures"], list)

    def test_missing_file_error(self):
        """Graceful error on missing report file."""
        mod = _load_compare_module()
        with pytest.raises((FileNotFoundError, SystemExit)):
            mod.load_report("/nonexistent/path/report.json")
