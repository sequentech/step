---
title: Verify the Bulletin Board
sidebar_position: 6
description: "Run Braid on the restored bulletin board to check public keys, mix evidence and references in published plaintext records."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Verify the Bulletin Board

This procedure checks the tally evidence on the restored bulletin board. The auditor does it.
Braid checks signatures, configuration, public keys, mix evidence and references in published plaintext records.
Its decryption routine checks proofs and recomputes plaintexts when it runs.
See [Versions](../01-before-you-start.md#versions) for the decryption completion requirement before reporting verified decryption.

The ballot audit checks the encrypted ballots that went into the tally.
Braid checks the cryptographic evidence that follows those inputs. You must do both parts.

## Before you start

- You did [Audit the Bundle](05-audit.md).
- The B3 server runs on the B3 copy at `127.0.0.1:50051`. The `shell` of the audit image starts
  it. Without the audit image, see [Without the Audit Image](01-without-kit.md#7-start-the-b3-server).
- You have the board name of the scope.

## 1. Find all batches on the board

The verifier checks every batch on the board. The board can hold the batches of other tallies of
the same election event.

1. Open a SQL session on the B3 copy:

   ```sh
   docker exec -it audit-b3-db psql -U audit_reader -d b3_copy
   ```

2. List the batches that have encrypted ballots. The B3 table of the board has the name of the
   board in lowercase, cut to 63 characters:

   ```sql
   SELECT DISTINCT batch FROM "<board table>" WHERE statement_kind = 'Ballots' ORDER BY batch;
   ```

3. Make sure that the list contains all batches of your scope.
4. Write the list in the audit notes.

**Expected result:** you have the complete list of batches on the board.

## 2. Run the verifier

1. Run the verifier with all batches of the board:

   ```sh
   verify --server-url http://127.0.0.1:50051 --board "<board name>" --batches 0,17,34
   ```

2. Read the exit status.
3. Read the summary. Write the number of passed checks and the number of all checks in the
   audit notes.

**Expected result:** the exit status is `0`. All required checks pass.

:::caution CAUTION
Do not leave out `--batches`. Without it, the verifier does not detect a missing batch that has no
ballots.
:::

## If there is a problem

| Problem | Action |
| --- | --- |
| Exit status not `0`, `required checks failed or were incomplete` | Read the summary. A failed check of the mix or of the decryption is a finding: keep the evidence and report it. |
| The verifier fails when you give only the batches of your scope. | The board holds other batches. Give the complete list of the board. |
| The verifier cannot connect. | Make sure that the B3 server runs and listens on `127.0.0.1:50051`. |

## Next

[Check the Cast-Vote History](07-history.md), or [Report and Remove the Copies](../02-procedures/04-close.md)
if you do not do the optional check.
