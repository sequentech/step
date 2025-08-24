#!/bin/sh
# Usage: ./script.sh <subcommand> [options]
# Subcommands: vote-cast, shell, sleep, tally
# For vote-cast: ./script.sh vote-cast --voting-url <value>
# If --voting-url is not provided, falls back to $VOTING_URL.

# Check for mandatory first argument (subcommand)
if [ $# -eq 0 ]; then
  echo "Error: Missing subcommand" >&2
  echo "Usage: $0 <subcommand> [options]" >&2
  echo "Subcommands: load-tool, vote-cast, shell, sleep, step-cli" >&2
  exit 1
fi

SUBCOMMAND="$1"
shift

VOTING_URL_VALUE="${VOTING_URL:-$LOADTESTING_VOTING_URL}"

# Validate subcommand
case "$SUBCOMMAND" in
  load-tool|vote-cast|shell|sleep|step-cli)
    ;;
  --help|-h)
    echo "Usage: $0 <subcommand> [options]"
    echo "Subcommands:"
    echo "  load-tool    Run the load-tool tool"
    echo "  vote-cast    Run vote casting load tests [--voting-url <value>]"
    echo "  shell        Start an interactive shell"
    echo "  sleep        Sleeps for an infinite amount of time"
    echo "  step-cli     Run step-cli"
    echo ""
    echo "For vote-cast subcommand:"
    echo "  --voting-url <value>    Voting URL (falls back to \$LOADTESTING_VOTING_URL if not provided)"
    exit 0
    ;;
  *)
    echo "Error: Unknown subcommand '$SUBCOMMAND'" >&2
    echo "Valid subcommands: load-tool, vote-cast, shell, sleep, step-cli" >&2
    exit 1
    ;;
esac

# Parse CLI args (only for vote-cast subcommand)
if [ "$SUBCOMMAND" = "load-tool" ]; then
  /opt/sequent-step/load-tool "$@"
elif [ "$SUBCOMMAND" = "vote-cast" ]; then
  while [ $# -gt 0 ]; do
    case "$1" in
      --voting-url)
        shift
        if [ $# -eq 0 ]; then
          echo "Error: --voting-url requires a value" >&2
          exit 1
        fi
	export LOADTESTING_VOTING_URL="$1"
        ;;
      --voting-url=*)
        export LOADTESTING_VOTING_URL="${1#--voting-url=}"
        ;;
      --help|-h)
        echo "Usage: $0 vote-cast [--voting-url <value>]"
        echo "       Falls back to \$LOADTESTING_VOTING_URL if --voting-url is not provided."
        exit 0
        ;;
      *)
        echo "Unknown option for vote-cast: $1" >&2
        exit 1
        ;;
    esac
    shift
  done
  cd /nightwatch && npm run test  
elif [ "$SUBCOMMAND" = "shell" ]; then
  bash "$@"
elif [ "$SUBCOMMAND" = "sleep" ]; then
  sleep infinity
elif [ "$SUBCOMMAND" = "step-cli" ]; then
  /opt/sequentech/step-cli "$@"
fi
