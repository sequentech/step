#!/bin/sh

# SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Usage: ./script.sh <subcommand> [options]
# Subcommands: e2e, load-tool, vote-cast, shell, sleep, tally
# For vote-cast: ./script.sh vote-cast --voting-url <value>
# If --voting-url is not provided, falls back to $VOTING_URL or $LOADTESTING_VOTING_URL.

# Check for mandatory first argument (subcommand)
if [ $# -eq 0 ]; then
  echo "Error: Missing subcommand" >&2
  echo "Usage: $0 <subcommand> [options]" >&2
  echo "Subcommands: e2e, load-tool, vote-cast, shell, sleep, step-cli" >&2
  exit 1
fi

SUBCOMMAND="$1"
shift

# Validate subcommand
case "$SUBCOMMAND" in
  e2e|load-tool|vote-cast|shell|sleep|step-cli)
    ;;
  --help|-h)
    echo "Usage: $0 <subcommand> [options]"
    echo "Subcommands:"
    echo "  e2e          Run e2e nightwatch tests"
    echo "  load-tool    Run the load-tool tool"
    echo "  vote-cast    Run vote casting load tests [--voting-url <value>]"
    echo "  shell        Start an interactive shell"
    echo "  sleep        Sleeps for an infinite amount of time"
    echo "  step-cli     Run step-cli"
    echo ""
    echo "For vote-cast subcommand:"
    echo "  --voting-url <value>    Voting URL (falls back to \$VOTING_URL or \$LOADTESTING_VOTING_URL if not provided)"
    echo "  Other options are forwarded to /run_bg_voting.sh (e.g. --batches, --instances, --save-screenshots)."
    exit 0
    ;;
  *)
    echo "Error: Unknown subcommand '$SUBCOMMAND'" >&2
    echo "Valid subcommands: e2e,load-tool, vote-cast, shell, sleep, step-cli" >&2
    exit 1
    ;;
esac

if [ "$SUBCOMMAND" = "e2e" ]; then
  cd /nightwatch
  if [ $# -eq 0 ]; then
      exec npm exec nightwatch src/admin-portal src/voting-portal
  fi
  exec npm exec nightwatch "$@"
elif [ "$SUBCOMMAND" = "load-tool" ]; then
  exec /opt/sequent-step/load-tool "$@"
elif [ "$SUBCOMMAND" = "vote-cast" ]; then
  while [ $# -gt 0 ]; do
    case "$1" in
      --voting-url)
        shift
        if [ $# -eq 0 ]; then
          echo "Error: --voting-url requires a value" >&2
          exit 1
        fi
        export VOTING_URL="$1"
        ;;
      --voting-url=*)
        export VOTING_URL="${1#--voting-url=}"
        ;;
      --help|-h)
        echo "Usage: $0 vote-cast [--voting-url <value>] [--batches N] [--instances N] [...]"
        echo "       Falls back to \$VOTING_URL or \$LOADTESTING_VOTING_URL if --voting-url is not provided."
        exit 0
        ;;
      *)
        # Stop parsing here; forward the remaining args to run_bg_voting.sh
        break
        ;;
    esac
    shift
  done
  # Delegate to the orchestrator; it will consume VOTING_URL and remaining flags
  exec /run_bg_voting.sh "$@"
elif [ "$SUBCOMMAND" = "shell" ]; then
  exec bash "$@"
elif [ "$SUBCOMMAND" = "sleep" ]; then
  exec sleep infinity
elif [ "$SUBCOMMAND" = "step-cli" ]; then
  exec /opt/sequentech/step-cli "$@"
fi
