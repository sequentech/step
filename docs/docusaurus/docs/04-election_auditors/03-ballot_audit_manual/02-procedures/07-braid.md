---
title: Verify the Bulletin Board
sidebar_position: 7
description: "Run the Braid verifier on the restored bulletin board to check the mix and the decryption of each batch."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Verify the Bulletin Board

This procedure checks the cryptography of the tally on the restored bulletin board. The auditor
does it. The Braid verifier checks the signatures, the configuration, the mix and the decryption
of each batch.

The ballot audit and this check are different. The ballot audit checks the encrypted ballots that
went into the tally. It does not check the mix or the decryption. A change to the mix or to the
decrypted results is found only by this check. You must do both.

## Before you start

- You did [Audit the Bundle](06-audit.md).
- The B3 server runs on the B3 copy at `127.0.0.1:50051`. See
  [Restore the Database Copies](02-restore.md#6-start-the-b3-server).
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
4. Write the list in the audit record.

**Expected result:** you have the complete list of batches on the board.

## 2. Run the verifier

1. Run the verifier with all batches of the board:

   ```sh
   verify --server-url http://127.0.0.1:50051 --board "<board name>" --batches 0,17,34
   ```

2. Read the exit status.
3. Read the summary. Write the number of passed checks and the number of all checks in the
   audit record.

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

[Check the Cast-Vote History](08-history.md), or [Report and Remove the Copies](09-close.md)
if you do not do the optional check.
