#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequent.io>
# SPDX-License-Identifier: AGPL-3.0-only
# Explicit entry point for an already provisioned COMELEC/OVCS receiver.
set -euo pipefail
umask 077
[[ $# -ge 6 && ( $1 = check || $1 = up ) ]] || {
    echo "usage: compose-receiver.sh {check|up} PROJECT PROJECT_DIRECTORY BASE_ENV APPROVED_SITE_FILE BASE_COMPOSE [BASE_COMPOSE ...]" >&2
    exit 2
}
operation=$1 project=$2 project_dir=$3 env_file=$4 site_file=$5
shift 5
[[ $project =~ ^[a-z0-9][a-z0-9_-]*$ && -d $project_dir && -f $env_file ]] || exit 2
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
scratch=$(mktemp -d)
trap 'rm -f "$scratch/base.json" "$scratch/resolved.json" "$scratch/again.json" "$scratch/render.err" "$scratch/site.json"; rmdir "$scratch"' EXIT
dc=(docker compose --project-name "$project" --project-directory "$project_dir")
sources=()
envs=(--env-file "$env_file")
while [[ $# != 0 ]]; do
    if [[ $1 = --env-file ]]; then
        [[ $# -ge 2 && -f $2 ]] || exit 2
        envs+=(--env-file "$2")
        shift 2
    else
        [[ -f $1 ]] || exit 2
        sources+=(-f "$1")
        shift
    fi
done
(( ${#sources[@]} > 0 ))
python3 "$here/check-compose.py" --site "$site_file" >"$scratch/site.json"
# Never print the (potentially credential-bearing) expanded Compose spec or errors.
if ! "${dc[@]}" "${envs[@]}" "${sources[@]}" config --format json     >"$scratch/base.json" 2>"$scratch/render.err"; then
    echo "Existing receiver Compose could not render." >&2
    exit 1
fi
if ! "${dc[@]}" --env-file /dev/null -f "$scratch/base.json" -f "$scratch/site.json" \
    config --format json >"$scratch/resolved.json" 2>"$scratch/render.err"; then
    echo "Receiver Compose could not render with the required site inputs." >&2
    exit 1
fi
# Fail closed if parsing the frozen result performs a second interpolation or
# otherwise reconstructs a model different from the one we validated.
if ! "${dc[@]}" --env-file /dev/null -f "$scratch/resolved.json" config --format json     >"$scratch/again.json" 2>"$scratch/render.err"; then
    echo "Frozen receiver Compose could not be safely reopened." >&2
    exit 1
fi
python3 "$here/check-compose.py" "$scratch/resolved.json" "$scratch/base.json" "$scratch/again.json" "$scratch/site.json"
if [[ $operation = up ]]; then
    # Use the already verified, fully resolved graph and existing images. Do
    # not install dependencies, start unrelated nodes, or alter network ACLs.
    "${dc[@]}" --env-file /dev/null -f "$scratch/resolved.json" \
        up -d --no-deps --no-build --pull never windmill beat harvest
fi
