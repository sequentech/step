#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
#
# remote-bench.sh -- the part of a bench-ec2.sh session that runs ON the
# instance (as root, via SSM). It downloads the pinned source tarball(s) from
# the session's S3 prefix, builds, runs bench.sh, optionally runs an
# interleaved before/after of the five targets against a baseline commit,
# uploads bench-results/ plus a machine header, and finally schedules its own
# shutdown (the instance is launched with shutdown-behavior=terminate).
#
#   remote-bench.sh SESSION BUCKET SHA [BASE_SHA]
#
# Environment: CELLS, REPS (bench.sh's grid); DIFF_CELLS (before/after grid,
# default "10000:2 100000:2"), DIFF_REPS (default 3).
set -euo pipefail

SESSION="$1"; BUCKET="$2"; SHA="$3"; BASE_SHA="${4:-}"
export PATH=/root/.cargo/bin:/usr/local/bin:$PATH
export CARGO_TERM_COLOR=never
# A reference session is about the targets; the criterion guidance benches are
# design inputs already recorded in PERFORMANCE.md, so they are off unless asked.
export GUIDANCE="${GUIDANCE:-0}"
DIFF_CELLS="${DIFF_CELLS:-10000:2 100000:2}"
DIFF_REPS="${DIFF_REPS:-3}"
S3="s3://$BUCKET/$SESSION"
WORK=/work
RESULTS=/work/results
mkdir -p "$WORK" "$RESULTS"

log() { printf '[remote %s] %s\n' "$(date -u +%H:%M:%S)" "$*"; }

log "waiting for the user-data bootstrap to finish"
until [ -f /var/tmp/wbraid-bootstrap-done ]; do sleep 5; done

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
    echo "# cpu:           $(lscpu | awk -F: '/Model name/ {gsub(/^ +/, "", $2); print $2}')"
    echo "# vcpus:         $(nproc)   threads/core: $(lscpu | awk -F: '/Thread\(s\) per core/ {gsub(/ /, "", $2); print $2}')"
    echo "# kernel:        $(uname -r)"
    echo "# rustc:         $(rustc --version)"
} | tee "$RESULTS/machine.txt"

# --- source: the exact commits, straight from S3 --------------------------------
fetch_src() { # fetch_src SHA DIR
    mkdir -p "$2"
    aws s3 cp "$S3/src-$1.tar.gz" - --only-show-errors | tar -xz -C "$2"
}
log "fetching source $SHA"
fetch_src "$SHA" "$WORK/cur"
CUR="$WORK/cur/packages/wbraid"

# --- bench.sh: builds untimed first, then criterion benches + the targets grid --
log "running bench.sh (CELLS='${CELLS:-<default>}' REPS='${REPS:-<default>}' GUIDANCE=$GUIDANCE)"
( cd "$CUR" && bash bench.sh )
cp "$CUR"/bench-results/*.txt "$RESULTS/"

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
    ( cd "$BASE" && cargo build --release -p vsc --example targets >/dev/null 2>&1 )
    DIFF="$RESULTS/differential-$BASE_SHA-vs-$SHA.csv"
    echo "tree,count,width,prove_ms,verify_ms,partial_decrypt_ms,combine_ms,ny_strip_ms,sizeof_bytes,ser_bytes" > "$DIFF"
    log "interleaved before/after over '$DIFF_CELLS' x $DIFF_REPS reps"
    for cell in $DIFF_CELLS; do
        n="${cell%%:*}"; w="${cell##*:}"
        for r in $(seq 1 "$DIFF_REPS"); do
            echo "base,$("$BASE/target/release/examples/targets" "$n" "$w" 2>/dev/null)" | tee -a "$DIFF"
            echo "curr,$("$CUR/target/release/examples/targets" "$n" "$w" 2>/dev/null)" | tee -a "$DIFF"
        done
    done
fi

# --- upload, then end the instance -----------------------------------------------
log "uploading results to $S3/results/"
aws s3 cp --recursive "$RESULTS/" "$S3/results/" --only-show-errors
log "done; shutting down in 2 minutes (the instance terminates on shutdown)"
shutdown -h +2 "wbraid-bench session complete"
