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
4. It compares the selected ciphertexts with each batch, including the expected number of copies of each.

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
| `selected` | The selected ballot matches the checked encrypted tally inputs. The report lists its matching positions. | `0` |
| `excluded` | The receipt appears in the selection file, but not among the selected ballots. The lookup reports the stored reason labels. | `3` |
| `missing` | The receipt was not found in the selected bundle or the selection records searched. Check `selection_searched` before drawing a conclusion. | `3` |
| `ambiguous` | More than one selected cast row has this receipt. Duplicate selected receipts make the audit fail. | `1` |

For an `excluded` or `missing` lookup outcome, exit status `3` comes from the lookup. The bundle check itself passed.
The receipt is not among the selected ballots of the checked inputs.
See [Read the Results](../03-read-the-results.md) for the stored reason labels and the limits of each outcome.

:::danger WARNING
Do not give a voter the batch and position of the ballot if the report lists the batch in
`privacy_warnings`. In a small batch, the position can show how a person voted.
:::

**Expected result:** you know whether the receipt matches a selected tally input, or which stored label explains its absence.

## If there is a problem

| Problem | Action |
| --- | --- |
| Exit status `1`, `manifest digest mismatch` or `configuration digest mismatch` | The bundle is not the bundle in the audit notes. Prepare it again from the evidence. Do not record new pins from a changed bundle. |
| Exit status `1`, `tally input multiset mismatch` | The encrypted ballots on the bulletin board are not the selected ballots. Keep the evidence and report it. |
| Exit status `3`, `missing B3 batch` | The bundle does not contain a batch of the scope. Check the export. |
| The receipt shows `missing`, but the voter has it. | The ballot can be in another area or another election. Ask for the area of the voter and audit that scope. |

## Next

[Verify the Bulletin Board](06-braid.md)
