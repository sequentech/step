#!/bin/sh
# Usage: ./script.sh <subcommand> [options]
# Subcommands: vote-cast, tally
# For vote-cast: ./script.sh vote-cast --voting-url <value>
# If --voting-url is not provided, falls back to $VOTING_URL.

# Check for mandatory first argument (subcommand)
if [ $# -eq 0 ]; then
  echo "Error: Missing subcommand" >&2
  echo "Usage: $0 <subcommand> [options]" >&2
  echo "Subcommands: vote-cast, tally" >&2
  exit 1
fi

SUBCOMMAND="$1"
shift

# Validate subcommand
case "$SUBCOMMAND" in
  vote-cast|tally)
    ;;
  --help|-h)
    echo "Usage: $0 <subcommand> [options]"
    echo "Subcommands:"
    echo "  vote-cast    Run vote casting load tests [--voting-url <value>]"
    echo "  tally        Run tally load tests"
    echo ""
    echo "For vote-cast subcommand:"
    echo "  --voting-url <value>    Voting URL (falls back to \$VOTING_URL if not provided)"
    exit 0
    ;;
  *)
    echo "Error: Unknown subcommand '$SUBCOMMAND'" >&2
    echo "Valid subcommands: vote-cast, tally" >&2
    exit 1
    ;;
esac

VOTING_URL_VALUE=""

# Parse CLI args (only for vote-cast subcommand)
if [ "$SUBCOMMAND" = "vote-cast" ]; then
  while [ $# -gt 0 ]; do
    case "$1" in
      --voting-url)
        shift
        if [ $# -eq 0 ]; then
          echo "Error: --voting-url requires a value" >&2
          exit 1
        fi
        VOTING_URL_VALUE="$1"
        ;;
      --voting-url=*)
        VOTING_URL_VALUE="${1#--voting-url=}"
        ;;
      --help|-h)
        echo "Usage: $0 vote-cast [--voting-url <value>]"
        echo "       Falls back to \$VOTING_URL if --voting-url is not provided."
        exit 0
        ;;
      *)
        echo "Unknown option for vote-cast: $1" >&2
        exit 1
        ;;
    esac
    shift
  done
elif [ "$SUBCOMMAND" = "tally" ]; then
  # Handle tally-specific arguments here if needed in the future
  while [ $# -gt 0 ]; do
    case "$1" in
      --help|-h)
        echo "Usage: $0 tally"
        echo "       Run tally load tests"
        exit 0
        ;;
      *)
        echo "Unknown option for tally: $1" >&2
        exit 1
        ;;
    esac
    shift
  done
fi

VOTING_URL_VALUE="${VOTING_URL:-$LOADTESTING_VOTING_URL}"

# This environment variable is used by the nightwatch tests
export VOTING_URL="${VOTING_URL_VALUE}"
