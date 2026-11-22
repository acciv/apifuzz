"""Анализ JSONL-результатов, которые записал Go-раннер."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

VERSION = "0.1.0"

SEVERITY_ORDER = {"low": 1, "medium": 2, "high": 3}


def load_results(path: str | Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with open(path, encoding="utf-8") as stream:
        for line in stream:
            if not line.strip():
                continue
            try:
                row = json.loads(line)
            except ValueError:
                continue
            if isinstance(row, dict):
                rows.append(row)
    return rows


def _expected(row: dict[str, Any]) -> set[int]:
    values = row.get("expected_status") or []
    out = set()
    for value in values:
        try:
            out.add(int(value))
        except (TypeError, ValueError):
            continue
    return out


def analyze(results: list[dict[str, Any]], slow_ms: float = 1000.0) -> dict[str, Any]:
    """Возвращает сводку и список находок по результатам прогона."""
    findings: list[dict[str, Any]] = []
    counts = {"high": 0, "medium": 0, "low": 0}

    def add(severity: str, kind: str, row: dict[str, Any], message: str) -> None:
        counts[severity] += 1
        findings.append(
            {
                "severity": severity,
                "kind": kind,
                "name": row.get("name", ""),
                "method": row.get("method", ""),
                "path": row.get("path", ""),
                "query": row.get("query", ""),
                "status": row.get("status"),
                "duration_ms": row.get("duration_ms"),
                "message": message,
            }
        )

    for row in results:
        error = row.get("error")
        if error:
            add("high", "request_error", row, str(error))
            continue

        status = row.get("status")
        try:
            status_int = int(status)
        except (TypeError, ValueError):
            add("high", "bad_status", row, f"некорректный статус: {status}")
            continue

        server_error = 500 <= status_int < 600
        if server_error:
            add("high", "server_error", row, f"сервер ответил {status_int}")
        elif status_int not in _expected(row):
            add("medium", "undeclared_status", row, f"статус {status_int} не описан в спецификации")

        if row.get("kind") == "valid" and not (200 <= status_int < 300) and not server_error:
            add("high", "baseline_failed", row, f"базовый запрос вернул {status_int}")

        try:
            duration = float(row.get("duration_ms", 0))
        except (TypeError, ValueError):
            duration = 0.0
        if duration > slow_ms:
            add("low", "slow_response", row, f"ответ за {duration:.1f} мс")

    total = len(results)
    return {
        "version": VERSION,
        "summary": {
            "total": total,
            "high": counts["high"],
            "medium": counts["medium"],
            "low": counts["low"],
            "clean": counts["high"] == 0 and counts["medium"] == 0,
        },
        "findings": findings,
    }


def _should_fail(report: dict[str, Any], threshold: str) -> bool:
    if threshold == "none":
        return False
    limit = SEVERITY_ORDER[threshold]
    return any(
        SEVERITY_ORDER.get(item["severity"], 0) >= limit
        for item in report["findings"]
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Анализ результатов apifuzz")
    parser.add_argument("--results", required=True, help="results.jsonl")
    parser.add_argument("--out", help="куда записать JSON-отчёт")
    parser.add_argument("--slow-ms", type=float, default=1000.0, help="порог медленного ответа")
    parser.add_argument("--fail-on", choices=["none", "low", "medium", "high"], default="high")
    parser.add_argument("--quiet", action="store_true", help="только код возврата")
    args = parser.parse_args(argv)

    try:
        results = load_results(args.results)
    except OSError as error:
        print(f"apifuzz: {error}", file=sys.stderr)
        return 2

    report = analyze(results, args.slow_ms)
    if args.out:
        Path(args.out).write_text(
            json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )
    if not args.quiet:
        summary = report["summary"]
        print(
            f"total={summary['total']} high={summary['high']} "
            f"medium={summary['medium']} low={summary['low']}"
        )
        for item in report["findings"]:
            print(f"  {item['severity']:6} {item['kind']:18} {item['method']} {item['path']} — {item['message']}")
    return 1 if _should_fail(report, args.fail_on) else 0


if __name__ == "__main__":
    raise SystemExit(main())
