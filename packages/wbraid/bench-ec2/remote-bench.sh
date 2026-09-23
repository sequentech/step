#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
#
# remote-bench.sh -- the part of a bench-ec2.sh session that runs ON the
# instance (as root, via SSM). It downloads the pinned source tarball(s) from
# the session's S3 prefix, builds `targets` at the tip, runs the snapshot grid,
# optionally the criterion guidance benches, optionally an interleaved
# before/after against a baseline commit, uploads the results plus a machine
# header, and finally schedules its own shutdown (the instance is launched with
# shutdown-behavior=terminate).
#
# It owns the grid loops rather than delegating to the packaged commit's
# bench.sh: what a session measures must not depend on what an older commit's
# scripts happen to understand (the 2026-09-22 session of 185dbbede2 ran the
# guidance benches despite GUIDANCE=0 for exactly that reason). bench.sh and
# bench.ps1 remain the local tools.
#
#   remote-bench.sh SESSION BUCKET SHA [BASE_SHA]
#
# Environment: CELLS (snapshot grid, default "1000:2 10000:2 10000:5 100000:2
# 100000:5") and REPS (3); DIFF_CELLS (before/after grid, default "10000:2
# 100000:2") and DIFF_REPS (3); TALLY_CELLS (the global target's grid of
# "N:W:Q" cells, default "100000:2:3 100000:5:3"; empty skips it) and
# TALLY_REPS (3); GUIDANCE (default 0 -- the criterion guidance benches are
# design inputs recorded in PERFORMANCE.md, not part of a snapshot).
#
# Outputs, under the session's results/ prefix: snapshot-<sha>.csv,
# differential-<base>-vs-<sha>.csv (with a baseline), guidance-<sha>.txt (with
# GUIDANCE=1), machine.txt.
set -euo pipefail

SESSION="$1"; BUCKET="$2"; SHA="$3"; BASE_SHA="${4:-}"
export PATH=/root/.cargo/bin:/usr/local/bin:$PATH
export CARGO_TERM_COLOR=never
CELLS="${CELLS:-1000:2 10000:2 10000:5 100000:2 100000:5}"
REPS="${REPS:-3}"
DIFF_CELLS="${DIFF_CELLS:-10000:2 100000:2}"
DIFF_REPS="${DIFF_REPS:-3}"
TALLY_CELLS="${TALLY_CELLS-100000:2:3 100000:5:3}"
TALLY_REPS="${TALLY_REPS:-3}"
TALLY_HEADER="count,width,quorum,ser,strip_prod_ms,strip_ver_ms,prove_ms,verify_ms,partial_ms,combine_ms,ser_ms,t_ms,v_ms"
GUIDANCE="${GUIDANCE:-0}"
S3="s3://$BUCKET/$SESSION"
WORK=/work
RESULTS=/work/results
mkdir -p "$WORK" "$RESULTS"
CSV_HEADER="count,width,prove_ms,verify_ms,partial_decrypt_ms,combine_ms,ny_strip_ms,sizeof_bytes,ser_bytes"

log() { printf '[remote %s] %s\n' "$(date -u +%H:%M:%S)" "$*"; }

log "waiting for the user-data bootstrap to finish"
until [ -f /var/tmp/wbraid-bootstrap-done ]; do sleep 5; done
if [ -f /var/tmp/wbraid-bootstrap-FAILED ]; then
    log "ERROR: bootstrap failed -- see /var/log/cloud-init-output.log"
    exit 5
fi

# --- helpers ---------------------------------------------------------------------
fetch_src() { # fetch_src SHA DIR
    mkdir -p "$2"
    aws s3 cp "$S3/src-$1.tar.gz" - --only-show-errors | tar -xz -C "$2"
    # Second line of defence against CRLF from a Windows-side git archive:
    # bash dies on '\r', cargo does not care.
    find "$2" -name '*.sh' -exec sed -i 's/\r$//' {} +
}

build_targets() { # build_targets WBRAID_DIR
    ( cd "$1" && cargo build --release -p vsc --example targets >/dev/null 2>&1 )
}

# cell_line BIN N W -- one targets run; a failure aborts the session (set -e)
# rather than vanishing inside an echo.
cell_line() { "$1" "$2" "$3" 2>/dev/null; }

# run_grid BIN CELLS REPS OUT -- the snapshot loop, one CSV line per run.
run_grid() {
    local bin="$1" cells="$2" reps="$3" out="$4" cell n w r line
    for cell in $cells; do
        n="${cell%%:*}"; w="${cell##*:}"
        for r in $(seq 1 "$reps"); do
            line="$(cell_line "$bin" "$n" "$w")"
            echo "$line" | tee -a "$out"
        done
    done
}

# run_tally_grid BIN CELLS REPS OUT -- the global target's loop over "N:W:Q"
# cells; the stage breakdown the example prints on stderr goes to the log.
run_tally_grid() {
    local bin="$1" cells="$2" reps="$3" out="$4" cell n w q r line
    for cell in $cells; do
        IFS=: read -r n w q <<EOF
$cell
EOF
        for r in $(seq 1 "$reps"); do
            line="$("$bin" "$n" "$w" "$q")"
            echo "$line" | tee -a "$out"
        done
    done
}

# --- source: the exact commits, straight from S3 --------------------------------
log "fetching source $SHA"
fetch_src "$SHA" "$WORK/cur"
CUR="$WORK/cur/packages/wbraid"

# --- toolchain: the tarball carries the repo-root pin; make sure it is usable ----
TC="$(sed -n 's/^channel *= *"\([^"]*\)".*/\1/p' "$WORK/cur/rust-toolchain.toml" 2>/dev/null || true)"
if ! rustc --version >/dev/null 2>&1; then
    log "rustc unusable after bootstrap; installing toolchain ${TC:-stable}"
    rustup toolchain install "${TC:-stable}" --profile minimal
    rustup default "${TC:-stable}"
fi

# --- machine header: every number is quoted with these --------------------------
TOKEN="$(curl -sX PUT http://169.254.169.254/latest/api/token \
           -H 'X-aws-ec2-metadata-token-ttl-seconds: 300' || true)"
md() { curl -sH "X-aws-ec2-metadata-token: $TOKEN" "http://169.254.169.254/latest/meta-data/$1" || echo unknown; }
{
    echo "# wbraid EC2 benchmark session $SESSION"
    echo "# date:          $(date -u +%FT%TZ)"
    echo "# commit:        $SHA${BASE_SHA:+   baseline: $BASE_SHA}"
    echo "# instance-type: $(md instance-type)"
    echo "# ami:           $(md ami-id)"
    echo "# az:            $(md placement/availability-zone)"
    echo "# cpu:           $(lscpu | awk -F: '/^Model name/ {gsub(/^ +/, "", $2); print $2; exit}')"
    echo "# vcpus:         $(nproc)   threads/core: $(lscpu | awk -F: '/Thread\(s\) per core/ {gsub(/ /, "", $2); print $2}')"
    echo "# kernel:        $(uname -r)"
    echo "# rustc:         $(rustc --version)"
    echo "# grid:          CELLS='$CELLS' REPS=$REPS${BASE_SHA:+   DIFF_CELLS='$DIFF_CELLS' DIFF_REPS=$DIFF_REPS}   TALLY_CELLS='$TALLY_CELLS' TALLY_REPS=$TALLY_REPS   GUIDANCE=$GUIDANCE"
} | tee "$RESULTS/machine.txt"

# --- tip: build, then the snapshot grid --------------------------------------------
log "building the tip's targets"
build_targets "$CUR"
TIP_BIN="$CUR/target/release/examples/targets"

SNAP="$RESULTS/snapshot-$SHA.csv"
echo "$CSV_HEADER" > "$SNAP"
log "snapshot grid over '$CELLS' x $REPS reps"
run_grid "$TIP_BIN" "$CELLS" "$REPS" "$SNAP"

# --- the global target: the tally's critical path, per "N:W:Q" cell -------------
if [ -n "$TALLY_CELLS" ]; then
    if [ -f "$CUR/crates/vsc/examples/tally.rs" ]; then
        log "building the tip's tally"
        ( cd "$CUR" && cargo build --release -p vsc --example tally >/dev/null 2>&1 )
        TALLY="$RESULTS/tally-$SHA.csv"
        echo "$TALLY_HEADER" > "$TALLY"
        log "tally grid over '$TALLY_CELLS' x $TALLY_REPS reps"
        run_tally_grid "$CUR/target/release/examples/tally" "$TALLY_CELLS" "$TALLY_REPS" "$TALLY"
    else
        log "examples/tally.rs is not present in $SHA; tally grid skipped"
    fi
fi

# --- optional: the criterion guidance benches, straight from cargo ---------------
if [ "$GUIDANCE" = 1 ]; then
    GUIDE="$RESULTS/guidance-$SHA.txt"
    : > "$GUIDE"
    for b in parallel_tradeoff msm_strategy; do
        if [ -f "$CUR/crates/vsc/benches/$b.rs" ]; then
            log "criterion guidance bench: $b"
            echo "## $b (criterion)" >> "$GUIDE"
            ( cd "$CUR" && cargo bench -p vsc --bench "$b" 2>/dev/null ) \
                | grep -E "Benchmarking|time:" | grep -v -E "Warming|Collecting|Analyzing" >> "$GUIDE" \
                || log "guidance bench $b produced no results"
            echo >> "$GUIDE"
        else
            log "guidance bench $b is not present in $SHA; skipped"
        fi
    done
fi

# --- before/after vs a baseline commit -------------------------------------------
if [ -n "$BASE_SHA" ]; then
    log "fetching baseline $BASE_SHA"
    fetch_src "$BASE_SHA" "$WORK/base"
    BASE="$WORK/base/packages/wbraid"
    # examples/targets.rs uses only fork-point public APIs precisely so it can
    # be dropped into a baseline that predates it (PERFORMANCE.md §6).
    if [ ! -f "$BASE/crates/vsc/examples/targets.rs" ]; then
        cp "$CUR/crates/vsc/examples/targets.rs" "$BASE/crates/vsc/examples/targets.rs"
    fi
    log "building the baseline's targets"
    build_targets "$BASE"
    BASE_BIN="$BASE/target/release/examples/targets"
    DIFF="$RESULTS/differential-$BASE_SHA-vs-$SHA.csv"
    echo "tree,$CSV_HEADER" > "$DIFF"
    log "interleaved before/after over '$DIFF_CELLS' x $DIFF_REPS reps"
    for cell in $DIFF_CELLS; do
        n="${cell%%:*}"; w="${cell##*:}"
        for r in $(seq 1 "$DIFF_REPS"); do
            line="$(cell_line "$BASE_BIN" "$n" "$w")"; echo "base,$line" | tee -a "$DIFF"
            line="$(cell_line "$TIP_BIN" "$n" "$w")";  echo "curr,$line" | tee -a "$DIFF"
        done
    done
fi

# --- upload, then end the instance -----------------------------------------------
log "uploading results to $S3/results/"
aws s3 cp --recursive "$RESULTS/" "$S3/results/" --only-show-errors
log "done; shutting down in 2 minutes (the instance terminates on shutdown)"
shutdown -h +2 "wbraid-bench session complete"
