import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from apifuzz.report import analyze


def row(status=200, kind="valid", error="", duration=10, expected=(200, 400), path="/users"):
    return {
        "name": "case",
        "method": "GET",
        "path": path,
        "query": "",
        "kind": kind,
        "expected_status": list(expected),
        "status": status,
        "error": error,
        "duration_ms": duration,
    }


class ReportTest(unittest.TestCase):
    def test_clean_run(self):
        report = analyze([row()])
        self.assertTrue(report["summary"]["clean"])
        self.assertEqual(report["summary"]["high"], 0)

    def test_server_error_is_high(self):
        report = analyze([row(status=500)])
        self.assertEqual(report["summary"]["high"], 1)
        self.assertEqual(report["findings"][0]["kind"], "server_error")

    def test_undeclared_status_is_medium(self):
        report = analyze([row(status=418)])
        self.assertEqual(report["summary"]["medium"], 1)
        self.assertEqual(report["findings"][0]["kind"], "undeclared_status")

    def test_error_is_high(self):
        report = analyze([row(status=0, error="dial tcp: refused")])
        self.assertEqual(report["findings"][0]["kind"], "request_error")

    def test_baseline_failure(self):
        report = analyze([row(status=404, expected=(200, 404))])
        self.assertTrue(any(f["kind"] == "baseline_failed" for f in report["findings"]))

    def test_slow_response(self):
        report = analyze([row(duration=2500)], slow_ms=1000)
        self.assertTrue(any(f["kind"] == "slow_response" for f in report["findings"]))


if __name__ == "__main__":
    unittest.main()
