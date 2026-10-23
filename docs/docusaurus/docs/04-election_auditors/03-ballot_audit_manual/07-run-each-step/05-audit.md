---
title: Audit the Bundle
sidebar_position: 5
description: "Check the audit bundle independently against the encrypted ballots on the bulletin board, and look up the receipts of voters."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Audit the Bundle

This procedure checks the audit bundle a second time, independently of the preparation. It also
looks up the receipts of voters. The auditor does it.

The command `ballot-audit` does these checks:

1. It calculates the receipt of each selected ballot again.
2. It checks the signed electoral log record of each selected ballot.
3. It checks the signatures of the configuration and of the encrypted ballots on the bulletin
   board.
4. It compares the selected ballots with the encrypted ballots of each batch, with the number
   of times that each ballot counts.

## Before you start

- You did [Prepare the Audit Bundle](04-prepare.md).
- `MANIFEST_SHA512` and `CONFIGURATION_SHA512` are set from the audit notes.

## 1. Audit the bundle

1. Go to the audit directory:

   ```sh
   cd ~/audit
   ```

2. Run the audit and keep the report:

   ```sh
   ballot-audit \
     --manifest prepared-audit/manifest.json --manifest-sha512 "$MANIFEST_SHA512" \
     --cast-votes prepared-audit/cast-votes.json \
     --configuration prepared-audit/configuration.borsh \
     --configuration-sha512 "$CONFIGURATION_SHA512" \
     --messages prepared-audit/messages.json \
     --electoral-log prepared-audit/electoral-log.json > audit-report.json
   ```

3. Read the exit status.
4. Write the exit status and the `checked_*` counts of `audit-report.json` in the audit notes.

**Expected result:** the exit status is `0`. The report shows the number of checked ballots,
batches and electoral log records.

## 2. Look up a receipt

Do this step when a voter, an observer or the election authority asks about one receipt.

1. Get the full receipt from the voter.
2. Run the audit with the receipt and the selection file:

   ```sh
   ballot-audit \
     --manifest prepared-audit/manifest.json --manifest-sha512 "$MANIFEST_SHA512" \
     --cast-votes prepared-audit/cast-votes.json \
     --configuration prepared-audit/configuration.borsh \
     --configuration-sha512 "$CONFIGURATION_SHA512" \
     --messages prepared-audit/messages.json \
     --electoral-log prepared-audit/electoral-log.json \
     --ballot-id "<receipt>" --selection prepared-audit/selection.json > lookup.json
   ```

3. Read the exit status.
4. Read the field `lookup.outcome` of `lookup.json`.

The command checks all ballots each time. The receipt only filters what the report shows.

| `lookup.outcome` | Meaning | Exit status |
| --- | --- | --- |
| `selected` | The tally counted the ballot. The report gives its batch and its position. | `0` |
| `excluded` | The ballot is stored, but the tally did not count it. The report gives the reason. | `3` |
| `missing` | The ballot is not in the scope. | `3` |
| `ambiguous` | More than one counted ballot has this receipt. The preparation refuses this case, so it shows only with a bundle from another source. | `1` |

An exit status `3` for a lookup is not a failure of the audit. It tells you that the tally did not
count this receipt. See [Read the Results](../03-read-the-results.md) for the reasons.

:::danger WARNING
Do not give a voter the batch and position of the ballot if the report lists the batch in
`privacy_warnings`. In a small batch, the position can show how a person voted.
:::

**Expected result:** you know whether the tally counted the ballot, and why not if it did not.

## If there is a problem

| Problem | Action |
| --- | --- |
| Exit status `1`, `manifest digest mismatch` or `configuration digest mismatch` | The bundle is not the bundle in the audit notes. Prepare it again from the evidence. Do not record new pins from a changed bundle. |
| Exit status `1`, `tally input multiset mismatch` | The encrypted ballots on the bulletin board are not the selected ballots. Keep the evidence and report it. |
| Exit status `3`, `missing B3 batch` | The bundle does not contain a batch of the scope. Check the export. |
| The receipt shows `missing`, but the voter has it. | The ballot can be in another area or another election. Ask for the area of the voter and audit that scope. |

## Next

[Verify the Bulletin Board](06-braid.md)
