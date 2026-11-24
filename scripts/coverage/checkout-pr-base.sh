#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

# The caller has checked out github.sha in head/ with both parents available.
set -euo pipefail
export GIT_NO_REPLACE_OBJECTS=1

fail() {
    echo "Cannot select the PR coverage base. $1" >&2
    exit 1
}
oid='^[0-9a-f]{40}$'
[[ $GITHUB_EVENT_NAME == pull_request && $PR_NUMBER =~ ^[1-9][0-9]*$ &&
   $GITHUB_REF == "refs/pull/$PR_NUMBER/merge" ]] || fail "Expected the event's PR merge ref."
[[ $GITHUB_SHA =~ $oid && $PR_HEAD_SHA =~ $oid ]] || fail "Expected full commit IDs."
# GitHub can omit merge_commit_sha. Its run SHA and merge ref are authoritative.
[[ -z ${PR_MERGE_SHA:-} || $PR_MERGE_SHA == "$GITHUB_SHA" ]] || fail "Conflicting merge IDs."

parents=$(git -C head rev-list --parents -n 1 HEAD --)
read -r merge base topic extra <<< "$parents"
[[ $merge == "$GITHUB_SHA" && $base =~ $oid && $topic == "$PR_HEAD_SHA" &&
   $base != "$topic" && -z $extra ]] || fail "Expected the run commit and exactly two parents, with the PR head second."
git -C head cat-file -e "${topic}^{commit}"
[[ ! -e base && ! -L base ]] || fail "The base destination already exists."
git -C head worktree add --detach ../base "$base"
printf 'Coverage base %s, tested merge %s, PR head %s\n' "$base" "$merge" "$topic"
