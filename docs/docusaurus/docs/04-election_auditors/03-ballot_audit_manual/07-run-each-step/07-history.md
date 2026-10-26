---
title: Check the Cast-Vote History
sidebar_position: 7
description: "Optional: compare the stored ballots of all areas of the election with the signed records of the electoral log."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Check the Cast-Vote History

This optional procedure compares all stored ballots of the election with the signed records of
the electoral log. It covers all areas, all statuses and all revotes, not only the selected ballots
of the scope. The auditor does it.

The command `audit-cast-history` finds two cases:

- A signed record without a stored ballot. The success record is written only after the ballot is
  stored, so the ballot was removed or replaced. This is a finding.
- A stored ballot without a signed record. The log write can fail, so this is not a finding. The
  command lists it, and the exit status stays `0`. If the tally counted it, it is a vote to audit.

The command does not select votes, does not check the voters and does not read the bulletin
board.

## Before you start

- You did [Export the Evidence](03-export.md). The electoral log export has no `--election-id`.
- `SCOPE_SHA512`, `CAST_VOTE_HISTORY_SHA512` and `ELECTORAL_LOG_SHA512` are set from the audit
  record.

## 1. Check the history

1. Go to the audit directory:

   ```sh
   cd ~/audit
   ```

2. Run the check and keep the report:

   ```sh
   audit-cast-history \
     --scope postgres-export/scope.json --scope-sha512 "$SCOPE_SHA512" \
     --cast-vote-history postgres-export/cast-vote-history.json \
     --cast-vote-history-sha512 "$CAST_VOTE_HISTORY_SHA512" \
     --electoral-log immudb-export/electoral-log.json \
     --electoral-log-sha512 "$ELECTORAL_LOG_SHA512" > history-report.json
   ```

3. Read the exit status.
4. Write the exit status and the coverage counts of `history-report.json` in the audit notes.

**Expected result:** the exit status is `0`. The `outcome` is `consistent`, or `success`
when some stored ballots have no signed record.

## 2. Read the groups

Each group of the report has a `classification`:

| Classification | Meaning | Action |
| --- | --- | --- |
| `matched`, `matched_repeated_logs` | The stored ballot has its signed record. | None. |
| `matched_ambiguous_row_attribution` | More than one stored ballot has the same signed record. | Write it in the audit report. |
| `matched_ambiguous_log_occurrences` | Different signed records describe the same ballot. | Write it in the audit report. |
| `hasura_row_without_signed_log` | A stored ballot has no signed record. If the tally counted it, it is a vote to audit. | Review it. See [Votes to audit](../03-read-the-results.md#votes-to-audit). |
| `signed_log_without_hasura_row` | A signed ballot has no stored ballot. | Keep the evidence and report it. |

**Expected result:** you know the classification of each group.

## If there is a problem

| Problem | Action |
| --- | --- |
| Exit status `1` with a report, `outcome: contradicted` | At least one signed ballot has no stored ballot. Read the `signed_log_without_hasura_row` groups and report them. |
| Exit status `1` without a report | A signature, a hash or a digest is wrong. Read the message. |
| Exit status `4`, foreign or rotated system keys | The electoral log was signed with more than one system key. The command supports one key. Write this in the audit report. |

## Next

[Report and Remove the Copies](../02-procedures/04-close.md)
