---
title: Prepare the Audit Bundle
sidebar_position: 4
description: "Select the ballots of the scope from the evidence, check them against the signed electoral log and the saved tally, and write the audit bundle."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Prepare the Audit Bundle

This procedure selects the ballots that the selection rules keep and writes them into an audit
bundle. The auditor does it. The command `prepare-ballot-audit` works only on files. It does not
connect to a database or to the network.

The command does these checks:

1. It checks the signature of each `CastVote` record of the election in the electoral log.
2. It selects each eligible voter's latest cast row marked valid in the application database.
3. It finds the signed record of each selected ballot.
4. It checks that no signed ballot of a selected voter was removed or replaced.
5. It checks that no later signed revote was left out.
6. It compares the selection with the counts that the tally saved.
7. It checks the bundle that it wrote, before it keeps it.

## Before you start

- You did [Export the Evidence](03-export.md).
- The seven digest variables are set: `SCOPE_SHA512` and the six of the evidence.
- The directory `~/audit/prepared-audit` does not exist.

## 1. Prepare the bundle

1. Go to the audit directory:

   ```sh
   cd ~/audit
   ```

2. Run the preparation:

   ```sh
   prepare-ballot-audit \
     --scope postgres-export/scope.json --scope-sha512 "$SCOPE_SHA512" \
     --cast-votes postgres-export/cast-votes.json --cast-votes-sha512 "$CAST_VOTES_SHA512" \
     --eligible-voters postgres-export/eligible-voters.json \
     --eligible-voters-sha512 "$ELIGIBLE_VOTERS_SHA512" \
     --board-messages postgres-export/board-messages.json \
     --board-messages-sha512 "$BOARD_MESSAGES_SHA512" \
     --electoral-log immudb-export/electoral-log.json \
     --electoral-log-sha512 "$ELECTORAL_LOG_SHA512" \
     --saved-mapping postgres-export/saved-mapping.json \
     --saved-mapping-sha512 "$SAVED_MAPPING_SHA512" \
     --cast-vote-history postgres-export/cast-vote-history.json \
     --cast-vote-history-sha512 "$CAST_VOTE_HISTORY_SHA512" \
     --output prepared-audit
   ```

3. Read the exit status.
4. Write the counts that the command shows in the audit notes.

:::caution CAUTION
Do not use `--without-cast-vote-history`. Without the history, the command cannot detect a
removed revote or a changed vote order.
:::

**Expected result:** the exit status is `0`. The directory `prepared-audit` contains
`manifest.json`, `configuration.borsh`, `cast-votes.json`, `messages.json`,
`electoral-log.json`, `anchors.json`, `provenance.json` and `selection.json`.

## 2. Record the pins of the bundle

1. Calculate the digests of the manifest and of the configuration:

   ```sh
   export MANIFEST_SHA512=$(shasum -a 512 prepared-audit/manifest.json | cut -d' ' -f1)
   export CONFIGURATION_SHA512=$(shasum -a 512 prepared-audit/configuration.borsh | cut -d' ' -f1)
   ```

2. Write the two digests in the audit notes.

:::danger WARNING
Do not use the digests in `anchors.json` as pins. The bundle wrote that file itself. A changed
bundle can contain changed anchors.
:::

**Expected result:** the audit notes have the digests of the manifest and of the configuration.

## If there is a problem

| Problem | Action |
| --- | --- |
| Exit status `1`, `scoped electoral-log signature verification failed` | A signed record is changed, or the system public key in the scope is wrong. Check the key with the election authority. If the key is correct, keep the evidence and report it. |
| Exit status `1`, `pinned public key hash differs from the board PublicKey statement` | The public key hash in the scope is wrong, or the key on the board was replaced. Check the hash with the election authority. |
| Exit status `1`, `a selected voter has a signed cast vote that no stored cast vote holds` | A stored ballot was removed or replaced. Keep the evidence and report it. |
| Exit status `1`, `a selected voter has a later signed revote ...` | The stored vote order disagrees with the signed log. If the times are seconds apart, ask the system operator to check the server clocks. Then report it. |
| Exit status `1`, `saved ... count differs ...` | The saved tally disagrees with the selection. A voter or a ballot changed after the tally, or a backup is from another moment. Read [Read the Results](../03-read-the-results.md). |
| Exit status `1`, `... ballot receipt(s) are shared by several selected cast votes` | Several selected cast rows share the same ballot receipt. Keep the evidence and report it. |
| Exit status `3`, `selected cast vote has no matching signed electoral-log evidence` | The signed record of a selected ballot is missing. The log write can fail. Report it as incomplete, not as a change. |
| Exit status `3`, `scoped in-progress cast vote blocks preparation` | A vote was still in progress at the backup. Get a backup from after the end of the voting period. |

See [Troubleshooting](../04-troubleshooting.md) for all messages.

## Next

[Audit the Bundle](05-audit.md)
