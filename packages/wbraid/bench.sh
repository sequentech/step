#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
#
# Controlled benchmark run for the MSM optimization campaign.
#
# Run this with the machine otherwise quiesced (no other heavy processes) to
# get authoritative numbers. It builds everything FIRST (untimed), then runs
# the timed benchmarks, teeing everything to a timestamped results file under
# bench-results/.
#
# Usage (from packages/wbraid, in git bash or a bash shell):
#   ./bench.sh
#
# Windows/PowerShell twin: bench.ps1 (same grid, same output format).
#
# The criterion benches (parallel_tradeoff, msm_strategy) self-calibrate; the
# scaling examples are run over a fixed cell grid, REPS times each, so the
# median can be taken. Adjust CELLS/REPS below to taste.

set -euo pipefail
cd "$(dirname "$0")"

REPS="${REPS:-3}"
SHUFFLE_CELLS="${SHUFFLE_CELLS:-1000:2 10000:2 10000:5 100000:2 100000:5}"
DECRYPT_CELLS="${DECRYPT_CELLS:-10000:2 10000:5 100000:2}"

mkdir -p bench-results
STAMP="$(date +%Y%m%d-%H%M%S)"
OUT="bench-results/bench-${STAMP}.txt"

log() { echo "$@" | tee -a "$OUT"; }

# Resolve an example binary (Windows appends .exe, Linux does not).
ex() {
  local base="target/release/examples/$1"
  if [ -x "${base}.exe" ]; then echo "${base}.exe"; else echo "${base}"; fi
}

log "# wbraid benchmark run ${STAMP}"
log "# commit: $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
log "# host cores: $(nproc 2>/dev/null || echo '?')"
log ""

# --- Build everything first (NOT timed) -------------------------------------
echo "building (untimed)..."
cargo build --release -p vsc --examples >/dev/null 2>&1
cargo bench  -p vsc --bench parallel_tradeoff --no-run >/dev/null 2>&1
cargo bench  -p vsc --bench msm_strategy      --no-run >/dev/null 2>&1
echo "build done; starting timed run."

# --- Criterion micro-benches (statistical) ----------------------------------
log "## parallel_tradeoff (criterion)"
cargo bench -p vsc --bench parallel_tradeoff 2>/dev/null \
  | grep -E "Benchmarking|time:" | grep -v "Warming|Collecting|Analyzing" | tee -a "$OUT"
log ""

log "## msm_strategy (criterion)"
cargo bench -p vsc --bench msm_strategy 2>/dev/null \
  | grep -E "Benchmarking|time:" | grep -v "Warming|Collecting|Analyzing" | tee -a "$OUT"
log ""

# --- Scaling sweeps (absolute, current tree) --------------------------------
log "## shuffle_scaling  (count,width,fold,prove_ms,verify_ms,sizeof,ser_bytes)"
for cell in $SHUFFLE_CELLS; do
  n="${cell%%:*}"; w="${cell##*:}"
  for r in $(seq 1 "$REPS"); do
    "$(ex shuffle_scaling)" "$n" "$w" 2>/dev/null | tee -a "$OUT"
  done
done
log ""

log "## decrypt_scaling  (count,width,strip_serial,strip_par,partial_decrypt,combine)"
for cell in $DECRYPT_CELLS; do
  n="${cell%%:*}"; w="${cell##*:}"
  for r in $(seq 1 "$REPS"); do
    "$(ex decrypt_scaling)" "$n" "$w" 2>/dev/null | tee -a "$OUT"
  done
done
log ""

log "# done -> $OUT"
