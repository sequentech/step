#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
#
# bench-ec2.sh -- run the wbraid benchmarks on a temporary EC2 instance.
#
# A fresh instance is quiet by construction, and the same instance type is the
# same hardware every session -- which is what makes numbers comparable across
# time (see BENCH-EC2.md). The instance cannot outlive a session: it is launched
# with instance-initiated-shutdown-behavior=terminate AND a shutdown timer in
# its user-data (the lifetime cap), this driver terminates it on every exit
# path, and `sweep` proves nothing tagged Project=wbraid-bench is left alive.
#
# Usage (from packages/wbraid, Git Bash on Windows or any POSIX shell):
#   ./bench-ec2.sh smoke                     # ~1 cent: t3.micro, bootstrap, uname, terminate, sweep
#   ./bench-ec2.sh session [REF] [BASE_REF]  # full benchmark at REF (default HEAD); with BASE_REF
#                                            # also an interleaved before/after of the five targets
#   ./bench-ec2.sh sweep                     # list anything tagged that is still alive (exit 1 if so)
#   ./bench-ec2.sh terminate                 # terminate the instance in .bench-ec2/state
#   ./bench-ec2.sh terminate --all-tagged    # terminate every non-terminated tagged instance
#
# Overrides (environment): AWS_PROFILE_NAME=wbraid-bench REGION=eu-west-1
#   BUCKET=wbraid-bench-bucket INSTANCE_PROFILE=wbraid-bench-instance-role
#   INSTANCE_TYPE=c7i.4xlarge LIFETIME_MIN=120 ROOT_GB=30 RUST_TOOLCHAIN=1.96.0
#   CELLS / REPS / DIFF_CELLS / DIFF_REPS / GUIDANCE (default 0 on EC2) pass through.
set -euo pipefail

PROFILE="${AWS_PROFILE_NAME:-wbraid-bench}"
REGION="${REGION:-eu-west-1}"
BUCKET="${BUCKET:-wbraid-bench-bucket}"
INSTANCE_PROFILE="${INSTANCE_PROFILE:-wbraid-bench-instance-role}"
INSTANCE_TYPE="${INSTANCE_TYPE:-c7i.4xlarge}"
LIFETIME_MIN="${LIFETIME_MIN:-120}"
ROOT_GB="${ROOT_GB:-30}"
RUST_TOOLCHAIN="${RUST_TOOLCHAIN:-1.96.0}"
TAG_PROJECT="wbraid-bench"

HERE="$(cd "$(dirname "$0")" && pwd)"
STATE_DIR="$HERE/.bench-ec2"
STATE="$STATE_DIR/state"
mkdir -p "$STATE_DIR"

log() { printf '[bench-ec2 %s] %s\n' "$(date -u +%H:%M:%S)" "$*" >&2; }
die() { log "ERROR: $*"; exit 1; }

# --- aws CLI: find it, and hand a Windows binary Windows paths ------------------
AWS_BIN="$(command -v aws || true)"
if [ -z "$AWS_BIN" ]; then
    for c in "${LOCALAPPDATA:-}/Programs/Amazon/AWSCLIV2/aws.exe" "/c/Program Files/Amazon/AWSCLIV2/aws.exe"; do
        [ -n "$c" ] && [ -x "$c" ] && AWS_BIN="$c" && break
    done
fi
[ -n "$AWS_BIN" ] || die "aws CLI not found on PATH (nor in the default Windows install locations)"
case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*) winpath() { cygpath -w "$1"; } ;;
    *) winpath() { printf '%s' "$1"; } ;;
esac
aws() { "$AWS_BIN" --profile "$PROFILE" --region "$REGION" "$@"; }

# --- helpers ---------------------------------------------------------------------
ami_id() {
    # Newest official Canonical Ubuntu 24.04 (noble) amd64 gp3 image in REGION.
    aws ec2 describe-images --owners 099720109477 \
        --filters "Name=name,Values=ubuntu/images/hvm-ssd-gp3/ubuntu-noble-24.04-amd64-server-*" \
                  "Name=state,Values=available" \
        --query 'sort_by(Images,&CreationDate)[-1].ImageId' --output text
}

state_instance() { if [ -f "$STATE" ]; then awk '{print $1}' "$STATE"; fi; }

instance_state() {
    aws ec2 describe-instances --instance-ids "$1" \
        --query 'Reservations[0].Instances[0].State.Name' --output text 2>/dev/null || echo "unknown"
}

ensure_no_live_instance() {
    local iid; iid="$(state_instance)"
    [ -z "$iid" ] && return 0
    case "$(instance_state "$iid")" in
        terminated|unknown|None) rm -f "$STATE" ;;
        *) die "instance $iid from a previous run is still alive -- run './bench-ec2.sh terminate' first" ;;
    esac
}

user_data_file() {
    # The lifetime cap is the FIRST line: with shutdown-behavior=terminate, the
    # instance ends itself even if everything else here (or this driver) fails.
    local f; f="$(mktemp)"
    cat > "$f" <<EOF
#!/bin/bash
shutdown -h +${LIFETIME_MIN} "wbraid-bench lifetime cap (${LIFETIME_MIN} min)"
export DEBIAN_FRONTEND=noninteractive
apt-get update -y
apt-get install -y build-essential pkg-config libssl-dev unzip curl
curl -sSL https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip -o /tmp/awscliv2.zip
unzip -q /tmp/awscliv2.zip -d /tmp && /tmp/aws/install
curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain ${RUST_TOOLCHAIN}
touch /var/tmp/wbraid-bootstrap-done
EOF
    printf '%s' "$f"
}

launch() {
    local type="${1:-$INSTANCE_TYPE}"
    ensure_no_live_instance
    local ami; ami="$(ami_id)"
    { [ -n "$ami" ] && [ "$ami" != "None" ]; } || die "could not resolve an Ubuntu 24.04 AMI in $REGION"
    local ud; ud="$(user_data_file)"
    log "launching $type from $ami (lifetime cap ${LIFETIME_MIN} min, root ${ROOT_GB} GB gp3, terminate on shutdown)"
    local iid
    iid="$(aws ec2 run-instances \
        --image-id "$ami" --instance-type "$type" --count 1 \
        --iam-instance-profile "Name=$INSTANCE_PROFILE" \
        --instance-initiated-shutdown-behavior terminate \
        --user-data "file://$(winpath "$ud")" \
        --metadata-options HttpTokens=required \
        --block-device-mappings "[{\"DeviceName\":\"/dev/sda1\",\"Ebs\":{\"VolumeSize\":${ROOT_GB},\"VolumeType\":\"gp3\",\"DeleteOnTermination\":true}}]" \
        --tag-specifications \
            "ResourceType=instance,Tags=[{Key=Project,Value=$TAG_PROJECT},{Key=Ephemeral,Value=true},{Key=Name,Value=wbraid-bench}]" \
            "ResourceType=volume,Tags=[{Key=Project,Value=$TAG_PROJECT},{Key=Ephemeral,Value=true}]" \
        --query 'Instances[0].InstanceId' --output text)"
    rm -f "$ud"
    { [ -n "$iid" ] && [ "$iid" != "None" ]; } || die "run-instances returned no instance id"
    echo "$iid $type $(date -u +%FT%TZ)" > "$STATE"
    log "instance $iid launched; waiting for it to run"
    aws ec2 wait instance-running --instance-ids "$iid"
    log "waiting for the SSM agent to come online"
    local i status=""
    for i in $(seq 1 60); do
        status="$(aws ssm describe-instance-information --filters "Key=InstanceIds,Values=$iid" \
                  --query 'InstanceInformationList[0].PingStatus' --output text 2>/dev/null || true)"
        [ "$status" = "Online" ] && break
        sleep 10
    done
    [ "$status" = "Online" ] || die "SSM agent on $iid never came online (does the instance role carry AmazonSSMManagedInstanceCore?)"
    log "$iid is online"
    printf '%s' "$iid"
}

json_escape() { printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g'; }

# ssm_run INSTANCE TIMEOUT_SECONDS LINE... -- runs the lines as one shell script
# on the instance (AWS-RunShellScript executes them under sh: keep them POSIX),
# polls to completion, prints the remote stdout/stderr tails. Returns non-zero
# unless the invocation status is Success.
ssm_run() {
    local iid="$1" timeout="$2"; shift 2
    local params cid first=1 line status
    params="$(mktemp)"
    {
        printf '{"commands":['
        for line in "$@"; do
            [ $first -eq 1 ] || printf ','
            first=0
            printf '"%s"' "$(json_escape "$line")"
        done
        printf '],"executionTimeout":["%s"]}' "$timeout"
    } > "$params"
    cid="$(aws ssm send-command --instance-ids "$iid" --document-name AWS-RunShellScript \
            --comment "wbraid-bench" --parameters "file://$(winpath "$params")" \
            --query 'Command.CommandId' --output text)"
    rm -f "$params"
    log "ssm command $cid sent; polling every 30s"
    while :; do
        sleep 30
        status="$(aws ssm get-command-invocation --command-id "$cid" --instance-id "$iid" \
                  --query 'Status' --output text 2>/dev/null || true)"
        { [ -z "$status" ] || [ "$status" = "None" ]; } && status="Pending"
        case "$status" in
            Pending|InProgress|Delayed) log "  $status" ;;
            *) break ;;
        esac
    done
    log "ssm command $cid finished: $status"
    echo "--- remote stdout (tail) ---"
    aws ssm get-command-invocation --command-id "$cid" --instance-id "$iid" --query 'StandardOutputContent' --output text | tail -c 6000
    echo "--- remote stderr (tail) ---"
    aws ssm get-command-invocation --command-id "$cid" --instance-id "$iid" --query 'StandardErrorContent' --output text | tail -c 3000
    [ "$status" = "Success" ]
}

package() {
    # package REF [BASE_REF] -> uploads src tarball(s) + remote-bench.sh; prints "SESSION SHA BASE_SHA"
    local ref="${1:-HEAD}" base="${2:-}"
    local top; top="$(git -C "$HERE" rev-parse --show-toplevel)"
    local sha bsha=""; sha="$(git -C "$top" rev-parse --short=10 "$ref")"
    [ -n "$base" ] && bsha="$(git -C "$top" rev-parse --short=10 "$base")"
    local session; session="$(date -u +%Y%m%d-%H%M%S)-$sha"
    local tmp; tmp="$(mktemp -d)"
    # Carry the repo-root toolchain pin so cargo on the instance selects 1.96.0.
    local extra="" tc
    for tc in rust-toolchain rust-toolchain.toml; do
        git -C "$top" cat-file -e "$ref:$tc" 2>/dev/null && extra="$extra $tc"
    done
    log "packaging $ref ($sha)${bsha:+ and baseline $base ($bsha)} -> s3://$BUCKET/$session/"
    # shellcheck disable=SC2086
    git -C "$top" archive --format=tar.gz -o "$tmp/src-$sha.tar.gz" "$ref" packages/wbraid $extra
    aws s3 cp "$(winpath "$tmp/src-$sha.tar.gz")" "s3://$BUCKET/$session/src-$sha.tar.gz" --only-show-errors
    if [ -n "$bsha" ]; then
        # shellcheck disable=SC2086
        git -C "$top" archive --format=tar.gz -o "$tmp/src-$bsha.tar.gz" "$base" packages/wbraid $extra
        aws s3 cp "$(winpath "$tmp/src-$bsha.tar.gz")" "s3://$BUCKET/$session/src-$bsha.tar.gz" --only-show-errors
    fi
    # Strip any CR before upload: the script runs under Linux bash, and a
    # Windows checkout with autocrlf could hand us CR-terminated lines.
    tr -d '\r' < "$HERE/bench-ec2/remote-bench.sh" > "$tmp/remote-bench.sh"
    aws s3 cp "$(winpath "$tmp/remote-bench.sh")" "s3://$BUCKET/$session/remote-bench.sh" --only-show-errors
    rm -rf "$tmp"
    printf '%s %s %s' "$session" "$sha" "$bsha"
}

collect() {
    local session="$1"
    local dest="$HERE/bench-results/ec2-$session"
    mkdir -p "$dest"
    aws s3 sync "s3://$BUCKET/$session/results/" "$(winpath "$dest")" --only-show-errors
    log "results -> $dest"
    ls -1 "$dest" | sed 's/^/  /' >&2
    # The bucket expires objects after a day; remove this session's now anyway.
    aws s3 rm "s3://$BUCKET/$session/" --recursive --only-show-errors || true
}

terminate() {
    local ids=""
    if [ "${1:-}" = "--all-tagged" ]; then
        ids="$(aws ec2 describe-instances --filters "Name=tag:Project,Values=$TAG_PROJECT" \
                "Name=instance-state-name,Values=pending,running,stopping,stopped" \
                --query 'Reservations[].Instances[].InstanceId' --output text)"
    else
        ids="$(state_instance)"
    fi
    if [ -z "$ids" ] || [ "$ids" = "None" ]; then log "nothing to terminate"; rm -f "$STATE"; return 0; fi
    log "terminating: $ids"
    # shellcheck disable=SC2086
    aws ec2 terminate-instances --instance-ids $ids \
        --query 'TerminatingInstances[].[InstanceId,CurrentState.Name]' --output text >&2
    # shellcheck disable=SC2086
    aws ec2 wait instance-terminated --instance-ids $ids
    rm -f "$STATE"
    log "terminated"
}

sweep() {
    log "sweep: anything tagged Project=$TAG_PROJECT still alive in $REGION?"
    local inst vols
    inst="$(aws ec2 describe-instances --filters "Name=tag:Project,Values=$TAG_PROJECT" \
            "Name=instance-state-name,Values=pending,running,shutting-down,stopping,stopped" \
            --query 'Reservations[].Instances[].[InstanceId,InstanceType,State.Name,LaunchTime]' --output text)"
    if [ -n "$inst" ]; then echo "LIVE INSTANCES:"; echo "$inst"; else echo "instances: none"; fi
    if vols="$(aws ec2 describe-volumes --filters "Name=tag:Project,Values=$TAG_PROJECT" \
                --query 'Volumes[].[VolumeId,State,Size]' --output text 2>&1)"; then
        if [ -n "$vols" ]; then echo "VOLUMES:"; echo "$vols"; else echo "volumes: none"; fi
    else
        echo "volumes: not checked (add ec2:DescribeVolumes to the CLI policy; root volumes are DeleteOnTermination anyway)"
        vols=""
    fi
    echo "bucket objects (auto-expire after 1 day):"
    aws s3 ls "s3://$BUCKET/" --recursive | tail -5 || true
    [ -z "$inst" ] && [ -z "$vols" ]
}

cleanup_trap() {
    local rc=$?
    trap - EXIT
    if [ -n "$(state_instance)" ]; then
        log "exit (rc=$rc): making sure the instance is terminated"
        terminate || true
    fi
    sweep || log "SWEEP FOUND LIVE RESOURCES -- investigate now"
    exit $rc
}

smoke() {
    trap cleanup_trap EXIT
    LIFETIME_MIN="${LIFETIME_MIN_SMOKE:-25}"
    local iid; iid="$(launch t3.micro)"
    log "smoke: waiting for the bootstrap, then identifying the machine"
    ssm_run "$iid" 900 \
        'until [ -f /var/tmp/wbraid-bootstrap-done ]; do sleep 5; done' \
        'uname -a; nproc; lscpu | grep -E "Model name|^CPU\(s\)"' \
        '/usr/local/bin/aws --version; /root/.cargo/bin/rustc --version' \
        "/usr/local/bin/aws s3 ls s3://$BUCKET/ >/dev/null && echo 'bucket access from the instance: ok'"
    log "smoke run complete"
}

session() {
    trap cleanup_trap EXIT
    local ref="${1:-HEAD}" base="${2:-}"
    local session sha bsha
    read -r session sha bsha < <(package "$ref" "$base")
    local iid; iid="$(launch)"
    log "session $session: running remote-bench.sh on $iid (lifetime cap ${LIFETIME_MIN} min)"
    # POSIX sh lines (AWS-RunShellScript runs them under sh). The benchmark's own
    # exit code is what the invocation reports, not the log upload's.
    ssm_run "$iid" $(( LIFETIME_MIN * 60 )) \
        'until [ -f /var/tmp/wbraid-bootstrap-done ]; do sleep 5; done' \
        "export PATH=/root/.cargo/bin:/usr/local/bin:\$PATH CELLS='${CELLS:-}' REPS='${REPS:-}' DIFF_CELLS='${DIFF_CELLS:-}' DIFF_REPS='${DIFF_REPS:-}' GUIDANCE='${GUIDANCE:-}'" \
        "aws s3 cp s3://$BUCKET/$session/remote-bench.sh /tmp/remote-bench.sh --only-show-errors" \
        "bash /tmp/remote-bench.sh '$session' '$BUCKET' '$sha' '$bsha' > /tmp/remote-bench.log 2>&1; rc=\$?" \
        "tail -n 25 /tmp/remote-bench.log" \
        "aws s3 cp /tmp/remote-bench.log s3://$BUCKET/$session/results/remote-bench.log --only-show-errors" \
        "exit \$rc" \
      || log "remote run reported failure -- collecting whatever was uploaded (see remote-bench.log)"
    collect "$session"
}

case "${1:-}" in
    smoke)      smoke ;;
    session)    shift; session "$@" ;;
    sweep)      sweep ;;
    terminate)  shift; terminate "$@" ;;
    launch)     launch "${2:-}" >/dev/null ;;
    *) sed -n '5,26p' "$0"; exit 2 ;;
esac
