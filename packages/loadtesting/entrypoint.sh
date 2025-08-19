#!/bin/sh
# Usage: ./script.sh --voting-url <value>
# If --voting-url is not provided, falls back to $VOTING_URL.

VOTING_URL_VALUE=""

# Parse CLI args
while [ $# -gt 0 ]; do
  case "$1" in
    --voting-url)
      shift
      if [ $# -eq 0 ]; then
        echo "Error: --url requires a value" >&2
        exit 1
      fi
      VOTING_URL_VALUE="$1"
      ;;
    --voting-url=*)
      VOTING_URL_VALUE="${1#--voting-url=}"
      ;;
    --help|-h)
      echo "Usage: $0 [--voting-url <value>]"
      echo "       Falls back to \$VOTING_URL if --voting-url is not provided."
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
  shift
done

VOTING_URL_VALUE="${VOTING_URL:-$LOADTESTING_VOTING_URL}"

# This environment variable is used by the nightwatch tests
export VOTING_URL="${VOTING_URL_VALUE}"
