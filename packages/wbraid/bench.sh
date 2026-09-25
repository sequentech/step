#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
#
# Local benchmark run for wbraid's cryptography (bash; PowerShell twin:
# bench.ps1, same grids, same output).
#
# What it measures (the two measurement programs in crates/vsc/examples/):
#
#   targets  The five cryptographic stages of a tally, each timed alone, for one
#            cell "N:W" -- N ciphertexts of width W: shuffle prove, shuffle
#            verify, one trustee's partial decryption, combine (verify every
#            partial and interpolate), and the first mix's Naor-Yung
#            verify-and-strip. One CSV line per run, milliseconds:
#              count,width,prove,verify,partial_decrypt,combine,ny_strip,sizeof,ser
#   tally    The global target: one whole tally replayed with real data flow for
#            a quorum of Q trustees, cell "N:W:Q", composed into its
#            critical-path latency T (what the tally takes end to end, each
#            trustee on its own machine) and V (the external verifier's path).
#            One CSV line per run, milliseconds, stage breakdown on stderr:
#              count,width,quorum,ser,strip_prod,strip_ver,prove,verify,partial,combine,ser_ms,t,v
#
# Each cell runs REPS times; read the median. Results are appended to a
# timestamped file under bench-results/ (git-ignored). Everything is built
# first, untimed; run with the machine otherwise idle.
#
# These are the local tools. Authoritative numbers come from a fixed EC2
# reference machine instead -- bench-ec2.sh, documented in BENCH-EC2.md -- and
# PERFORMANCE.md (Status) is generated from such a session.
#
# Usage, from packages/wbraid (Linux, the devcontainer, or Git Bash on Windows):
#
#   GUIDANCE=0 ./bench.sh                                   # targets + tally over the default grids
#   GUIDANCE=0 CELLS="100000:2" TALLY_CELLS="100000:2:3" REPS=5 ./bench.sh
#   GUIDANCE=0 TALLY_CELLS="" ./bench.sh                    # targets only
#   GUIDANCE=0 CELLS="" TALLY_CELLS="1000000:1:2" ./bench.sh    # tally only
#   ./bench.sh                                              # also the criterion guidance benches (slow)
#
# Or run the programs directly, once built (cargo build --release -p vsc --examples):
#
#   ./target/release/examples/targets 100000 2             # N W
#   ./target/release/examples/tally 100000 2 3             # N W Q
#   ./target/release/examples/tally 100000 2 3 --ser       # + encoding/decoding of every posted message
#
# Supported widths W: 1 2 3 5 10; quorums Q (tally): 2 3 4 5 7.
#
# GUIDANCE=1 (the default) first runs the criterion benches parallel_tradeoff
# and msm_strategy, which steer implementation choices and are recorded in
# PERFORMANCE.md; they are not part of a snapshot.

set -euo pipefail
cd "$(dirname "$0")"

REPS="${REPS:-3}"
CELLS="${CELLS-1000:2 10000:2 10000:5 100000:2 100000:5}"
TALLY_CELLS="${TALLY_CELLS-10000:2:3 100000:2:3}"
GUIDANCE="${GUIDANCE:-1}"

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
if [ "$GUIDANCE" = 1 ]; then
  cargo bench  -p vsc --bench parallel_tradeoff --no-run >/dev/null 2>&1
  cargo bench  -p vsc --bench msm_strategy      --no-run >/dev/null 2>&1
fi
echo "build done; starting timed run."

# --- Criterion micro-benches (statistical; guidance, not snapshot) ------------
if [ "$GUIDANCE" = 1 ]; then
  log "## parallel_tradeoff (criterion)"
  cargo bench -p vsc --bench parallel_tradeoff 2>/dev/null \
    | grep -E "Benchmarking|time:" | grep -v "Warming|Collecting|Analyzing" | tee -a "$OUT"
  log ""

  log "## msm_strategy (criterion)"
  cargo bench -p vsc --bench msm_strategy 2>/dev/null \
    | grep -E "Benchmarking|time:" | grep -v "Warming|Collecting|Analyzing" | tee -a "$OUT"
  log ""
else
  log "# guidance benches skipped (GUIDANCE=0)"
  log ""
fi

# --- Top-level target snapshot (absolute, current tree) ---------------------
log "## targets  (count,width,prove,verify,partial_decrypt,combine,ny_strip,sizeof,ser)"
for cell in $CELLS; do
  n="${cell%%:*}"; w="${cell##*:}"
  for r in $(seq 1 "$REPS"); do
    "$(ex targets)" "$n" "$w" 2>/dev/null | tee -a "$OUT"
  done
done
log ""

# --- The global target: one tally's critical path, per N:W:Q cell -----------
if [ -n "$TALLY_CELLS" ]; then
  log "## tally  (count,width,quorum,ser,strip_prod,strip_ver,prove,verify,partial,combine,ser_ms,t,v)"
  for cell in $TALLY_CELLS; do
    IFS=: read -r n w q <<EOF
$cell
EOF
    for r in $(seq 1 "$REPS"); do
      "$(ex tally)" "$n" "$w" "$q" 2>/dev/null | tee -a "$OUT"
    done
  done
  log ""
fi

log "# done -> $OUT"
