#!/usr/bin/env bash
# Flash the app and capture the defmt/RTT log for N seconds (default 10).
# Exit 0 if no panic/ERROR appeared in the log, 1 otherwise.
# Usage: scripts/run-capture.sh [seconds] [--release]
set -uo pipefail
cd "$(dirname "$0")/../firmware"
SECS="${1:-10}"; shift || true
LOG="../logs/run-$(date +%Y%m%d-%H%M%S).log"; mkdir -p ../logs
cargo build "$@" || exit 2
timeout --signal=INT "$SECS" cargo run "$@" 2>&1 | tee "$LOG"
echo "LOG: firmware/$LOG"
if grep -qE "panicked|ERROR|HardFault" "$LOG"; then echo "RESULT: FAIL"; exit 1; fi
echo "RESULT: PASS"
