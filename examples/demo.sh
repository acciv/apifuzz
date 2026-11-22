#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

PORT=19101
URL="http://127.0.0.1:$PORT"
RESULTS=/tmp/apifuzz_results.jsonl
REPORT=/tmp/apifuzz_report.json
BIN=/tmp/apifuzz-runner
rm -f "$RESULTS" "$REPORT"

cargo build --release --manifest-path rust/Cargo.toml >/dev/null
(cd go && go build -o "$BIN" ./cmd/runner)

PORT="$PORT" python3 examples/target_api.py >/tmp/apifuzz_target.log 2>&1 &
TARGET_PID=$!
cleanup() {
  kill "$TARGET_PID" 2>/dev/null || true
  wait "$TARGET_PID" 2>/dev/null || true
}
trap cleanup EXIT

curl -s --retry 30 --retry-connrefused --retry-delay 1 "$URL/users/sample" >/dev/null

"$BIN" -spec examples/openapi.json -base-url "$URL" -out "$RESULTS" -workers 4

export PYTHONPATH="$ROOT/python"
echo "--- report ---"
python3 -m apifuzz --results "$RESULTS" --out "$REPORT" --slow-ms 500 || true
echo "report saved: $REPORT"
