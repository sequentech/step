#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Run b4 against localstack, or clear everything it has accumulated.
# Bash twin of b4.ps1.
#
#   ./b4.sh                   # run the service
#   ./b4.sh --reset           # delete all stored data, then run
#   ./b4.sh --reset --no-run  # delete and stop
#
# An unknown flag is an error.
# Dev tool: emulator runs leave boards behind that nothing will open again.

set -euo pipefail

CYAN=$'\e[36m'
RED=$'\e[31m'
YELLOW=$'\e[33m'
GRAY=$'\e[90m'
RESET=$'\e[0m'

cd "$(dirname "$0")"

RESET_DATA=0
NO_RUN=0
for arg in "$@"; do
    case "$arg" in
        --reset | --clear) RESET_DATA=1 ;;
        --no-run) NO_RUN=1 ;;
        *)
            echo "unknown argument: $arg (expected --reset/--clear, --no-run)" >&2
            exit 2
            ;;
    esac
done

# The S3 endpoint. A pre-set AWS_ENDPOINT_URL wins; then WBRAID_S3_ENDPOINT_URL
# (.devcontainer/.env.development, exported by devenv). Without either: inside
# a compose project LocalStack is the `localstack` service on the project
# network (see localstack.sh), addressed by service name like every other dev
# service; natively it is a standalone container on localhost.
DEV_NET=$(docker inspect "$(hostname)" \
    --format '{{range $k, $v := .NetworkSettings.Networks}}{{println $k}}{{end}}' \
    2>/dev/null | head -n 1 || true)
if [ -z "${AWS_ENDPOINT_URL:-}" ]; then
    if [ -n "${WBRAID_S3_ENDPOINT_URL:-}" ]; then
        AWS_ENDPOINT_URL="$WBRAID_S3_ENDPOINT_URL"
    elif [ -n "$DEV_NET" ]; then
        AWS_ENDPOINT_URL="http://localstack:4566"
    else
        AWS_ENDPOINT_URL="http://localhost:4566"
    fi
fi
export AWS_ENDPOINT_URL
export AWS_ACCESS_KEY_ID="test"
export AWS_SECRET_ACCESS_KEY="test"
export AWS_REGION="us-east-1"
export S3_BUCKET_NAME="wbraid-messages"
export AWS_FORCE_PATH_STYLE="true"
export RUST_LOG="b4=info"

# Database URL points at the workspace root b4.db
export DATABASE_URL="sqlite:$(pwd)/b4.db?mode=rwc"

# aws CLI, or the official image when it isn't installed (the devcontainer):
# joined to the compose project network, it resolves http://localstack:4566
# like every other service.
if command -v aws >/dev/null 2>&1; then
    aws_cli() { aws "$@"; }
elif [ -n "$DEV_NET" ]; then
    aws_cli() {
        docker run --rm --network "$DEV_NET" \
            -e AWS_ACCESS_KEY_ID -e AWS_SECRET_ACCESS_KEY -e AWS_REGION \
            amazon/aws-cli "$@"
    }
else
    aws_cli() {
        echo "aws CLI not found on PATH" >&2
        return 127
    }
fi

if [ "$RESET_DATA" -eq 1 ]; then
    # Two stores, and both have to go. MAX_INLINE_MESSAGE_SIZE is 0, so every
    # message body is in S3 and sqlite holds only metadata and the key pointing
    # at it: dropping the database alone would orphan the bodies rather than
    # remove them, and the bucket would keep growing.
    #
    # This only makes sense with the service stopped. Unlike Windows, Linux
    # happily unlinks an open file, so instead of relying on a locked-file
    # error, refuse the reset if a b4v6 process is running.
    echo "${CYAN}Clearing b4 data...${RESET}"

    if pgrep -x b4v6 >/dev/null 2>&1; then
        echo "  ${RED}b4 appears to be running - stop the service and try again.${RESET}"
        echo "  ${GRAY}(nothing was cleared; S3 was left alone so it still matches the database)${RESET}"
        exit 1
    fi

    # The database goes first, and a failure here stops the whole reset: the
    # bucket must not be emptied while the database still points into it, or b4
    # comes back serving metadata for bodies that are gone -- worse than not
    # having cleared at all.
    if [ -e b4.db ]; then
        # b4.db* also catches -wal and -shm, which sqlite may have left.
        rm -f b4.db*
        echo "  ${GRAY}removed $(pwd)/b4.db${RESET}"
    else
        echo "  ${GRAY}no b4.db to remove${RESET}"
    fi

    # The bucket itself stays: localstack.sh recreates it idempotently and
    # reapplies the CORS configuration, so emptying it avoids that step.
    if aws_cli --endpoint-url="$AWS_ENDPOINT_URL" s3 rm "s3://$S3_BUCKET_NAME" --recursive >/dev/null 2>&1; then
        echo "  ${GRAY}emptied s3://$S3_BUCKET_NAME${RESET}"
    else
        echo "  ${YELLOW}could not empty s3://$S3_BUCKET_NAME - is localstack running?${RESET}"
    fi

    echo ""
    echo "${YELLOW}Browser data is separate and is not touched by this.${RESET}"
    echo "${YELLOW}In the emulator's tab: DevTools -> Application -> Storage -> Clear site data${RESET}"
    echo "${GRAY}(that clears both localStorage and the per-trustee IndexedDB stores)${RESET}"
    echo ""

    if [ "$NO_RUN" -eq 1 ]; then
        exit 0
    fi
fi

# Run the service from workspace root.
cargo run --bin b4v6 --release
