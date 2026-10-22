#!/usr/bin/env bash
# Full pipeline: fmt -> clippy -> host unit tests -> firmware build -> HIL tests.
# Usage: scripts/check.sh [--hil]   (--hil: board required, fail if no probe)
# Logs: logs/check-<timestamp>/<stage>.log ; last line prints a summary.
set -uo pipefail
cd "$(dirname "$0")/.."
ROOT=$PWD
REQUIRE_HIL=0; [[ "${1:-}" == "--hil" ]] && REQUIRE_HIL=1
LOG_DIR="$ROOT/logs/check-$(date +%Y%m%d-%H%M%S)"; mkdir -p "$LOG_DIR"
HIL_TIMEOUT="${HIL_TIMEOUT:-300}"
declare -a RESULTS=(); FAILED=0

stage() { # stage <name> <dir> <cmd...>
  local name=$1 dir=$2; shift 2
  echo "== STAGE: $name =="
  if (cd "$ROOT/$dir" && "$@") >"$LOG_DIR/$name.log" 2>&1; then
    RESULTS+=("$name=PASS")
  else
    RESULTS+=("$name=FAIL"); FAILED=1
    tail -n 40 "$LOG_DIR/$name.log"
  fi
}

stage fmt          logic    cargo fmt --check
stage fmt-fw       firmware cargo fmt --check
stage clippy       logic    cargo clippy --all-targets -- -D warnings
stage clippy-fw    firmware cargo clippy -- -D warnings
stage host-test    logic    cargo test
stage build        firmware cargo build
stage build-tests  firmware cargo test --no-run

if command -v probe-rs >/dev/null && probe-rs list 2>/dev/null | grep -qiE "st-?link|cmsis|j-?link"; then
  stage hil firmware timeout "$HIL_TIMEOUT" cargo test
elif [[ $REQUIRE_HIL == 1 ]]; then
  echo "== STAGE: hil == no debug probe found"; RESULTS+=("hil=FAIL(no-probe)"); FAILED=1
else
  RESULTS+=("hil=SKIP(no-probe)")
fi

echo "SUMMARY: ${RESULTS[*]}  logs=$LOG_DIR"
exit $FAILED
