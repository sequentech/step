---
title: Get the Scope and the Pins
sidebar_position: 3
description: "Find the tally to audit, write the scope file with the trusted keys and pin it with its SHA-512 digest."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Get the Scope and the Pins

This procedure writes the scope file. The scope tells the tools which tally, election, area and
contest to audit, and which keys to trust. The auditor does it with the election authority.

## Before you start

- You did [Restore the Database Copies](02-restore.md).
- The election authority has told you which tally, election, area and contest to audit.
- You have these two keys from the election authority, through a trusted channel that is not
  the backups:
  - The **electoral log system public key**: DER/SPKI, encoded in standard base64.
  - The **public key hash**: the SHA-512 digest of the election public key on the bulletin board,
    in 128 lowercase hexadecimal characters.

:::danger WARNING
Do not read the two keys from the copies. The audit uses them to check the copies. A key from the
copies cannot show that the copies were changed.
:::

## 1. Find the tally values

You can read the IDs from the Hasura copy. The export checks them against the saved tally.

1. Open a SQL session on the Hasura copy:

   ```sh
   docker exec -it audit-hasura-db psql -U audit_reader -d hasura_copy
   ```

2. List the contests of the tally:

   ```sql
   SELECT id, tally_session_id, election_id, area_id, contest_id, session_id,
          annotations->>'weight_bit_mask' AS weight_bit_mask
   FROM sequent_backend.tally_session_contest
   WHERE tally_session_id = '<tally ID>';
   ```

3. Find the row of the election, area and contest to audit. Write its values in the audit record.
4. Read the name of the event board:

   ```sql
   SELECT tenant_id, bulletin_board_reference->>'database_name' AS board_name
   FROM sequent_backend.election_event
   WHERE id = '<election event ID>';
   ```

5. Write the tenant ID and the board name in the audit record.

**Expected result:** you have all the IDs of the scope, the `session_id` and the board name.

## 2. Find the batches and the policy

1. Set the base batch. It is the `session_id` of the row.
2. Set the policy:
   - `unweighted`: each voter has one vote.
   - `weighted`: the voters have vote weights.
   - `delegated`: voters can give their vote to another voter.
3. Write the list of batches:
   - For `unweighted` and `delegated`, the list has only the base batch.
   - For `weighted`, add the base batch and each set bit `b` of `weight_bit_mask`. The batch of bit
     `b` is the base batch plus `b`.

:::note
A tally that combines vote weights and delegation is not supported. The export stops with exit
status `4`.
:::

**Expected result:** you have the policy and the complete list of batches.

## 3. Write the scope file

1. Make the file `~/audit/scope.json`. Use lowercase IDs. For a ballot that encrypts all contests
   together, set `contest_id` to `null`:

   ```json
   {
     "format_version": 1,
     "scope": {
       "tenant_id": "<tenant ID>",
       "election_event_id": "<election event ID>",
       "election_id": "<election ID>",
       "area_id": "<area ID>",
       "tally_session_id": "<tally ID>",
       "tally_session_contest_id": "<ID of the row>",
       "board_name": "<board name>",
       "contest_id": "<contest ID>",
       "base_batch": 17,
       "policy": "unweighted",
       "public_key_sha512": "<public key hash from the election authority>",
       "electoral_log_system_public_key": "<system public key from the election authority>"
     },
     "batches": [17]
   }
   ```

2. Calculate the digest of the scope file and keep it in a variable:

   ```sh
   export SCOPE_SHA512=$(shasum -a 512 ~/audit/scope.json | cut -d' ' -f1)
   ```

3. Write the digest in the audit record.

:::caution CAUTION
Do not change `scope.json` after this step. A change of one space changes the digest. If you must
change it, calculate and record the digest again.
:::

**Expected result:** `scope.json` exists and its digest is in the audit record.

## If there is a problem

| Problem | Action |
| --- | --- |
| The tally has no row for the area and the contest. | Make sure that you have the correct tally ID. A tally can cover only some areas. |
| `weight_bit_mask` is empty for a weighted tally. | The tally did not finish, or the backup is from before its end. Get a later backup. |
| The election authority cannot give the two keys. | Stop. Without trusted keys, the audit cannot detect changed copies. |

## Next

[Export the Evidence](04-export.md)
