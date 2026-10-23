---
title: Run the Audit
sidebar_position: 3
description: "Audit all areas and contests of one tally with the audit-tally command, read the results and record the digest of the audit record."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Run the Audit

This procedure audits all areas and contests of one tally with one command, `audit-tally`. The
auditor does it with the audit kit: `audit-env tally` runs `audit-tally` on the restored copies.
The command reads the copies, runs each check of the audit and writes an audit record and a
public summary.

The command does these steps:

1. It reads the tally from the Hasura copy and makes one scope for each area and contest.
2. It exports the electoral log of the election event from the ImmuDB copy.
3. For each scope, it exports the evidence, prepares the audit bundle and audits the bundle.
4. For each election, it checks the cast-vote history.
5. It runs the Braid verifier on all batches of the tally and of the bulletin board.

The command calculates the pin of each file that it writes and gives it to the next step. It
records all pins in `audit-record.json`. The digest of `audit-record.json` protects all of them.

## Before you start

- You did [Restore the Database Copies](02-restore.md). `audit-env up` showed that the copies are
  ready.
- You have these three values from the election authority, through a trusted channel that is not
  the backups:
  - The **tally ID**.
  - The **public key hash**: the SHA-512 digest of the election public key on the bulletin board,
    in 128 lowercase hexadecimal characters.
  - The **electoral log system public key**: DER/SPKI, encoded in standard base64.
- The directory `~/audit/results/tally` does not exist. The command makes it.

:::danger WARNING
Do not read the tally ID or the two keys from the copies. The audit uses them to check the copies.
A value from the copies cannot show that the copies were changed.
:::

## 1. Set the trusted values

1. Set the three values from the election authority:

   ```sh
   export TALLY_SESSION_ID=<tally ID>
   export PUBLIC_KEY_SHA512=<public key hash>
   export ELECTORAL_LOG_SYSTEM_PUBLIC_KEY=<electoral log system public key>
   ```

2. Write the three values in the audit notes.

**Expected result:** the three variables are set. The audit notes contain the three values.

## 2. Run the audit

1. Run the audit:

   ```sh
   audit-env tally --dir ~/audit \
     --tally-session-id "$TALLY_SESSION_ID" \
     --public-key-sha512 "$PUBLIC_KEY_SHA512" \
     --electoral-log-system-public-key "$ELECTORAL_LOG_SYSTEM_PUBLIC_KEY"
   ```

2. Read the exit status:

   ```sh
   echo $?
   ```

The audit kit gives `audit-tally` the connections file, the read-only ImmuDB account, the address
of the B3 server and, if there is one, the B3 blob directory. The output is in
`~/audit/results/tally`. To use another output directory in `~/audit/results`, add
`--output <name>`.

:::note
The command runs the steps one after the other. A large tally can take a long time.
:::

**Expected result:** the exit status is `0`. The command shows one line for each scope, the
overall status and the digest of the audit record:

```text
scopes/001: election <ID> area <ID> contest <ID>: Success
scopes/002: election <ID> area <ID> contest all: Success
Overall: Success
audit-record.json SHA-512: <digest>
```

`contest all` is a scope whose ballots encrypt all contests together.

## 3. Read the results

1. Read the exit status:

   | Exit status | Meaning |
   | --- | --- |
   | `0` | All steps passed. |
   | `1` | A step found a contradiction. |
   | `3` | Evidence is missing, or a step did not run. The audit is incomplete. |
   | `4` | An input was refused. |

2. Open `~/audit/results/tally/public-summary.json`. Read `status`, the `status` of each scope,
   of each `cast_vote_history` entry and of `braid`.
3. For each status that is not `success`, open `~/audit/results/tally/audit-record.json`. Find
   the steps of that scope or election in `steps`. Read their `exit_code` and `message`.
4. Find each message in [Troubleshooting](../04-troubleshooting.md).
5. Write the overall status and the status of each scope in the audit notes.

:::note
A step with the status `not_run` did not run because an earlier step did not pass. Correct the
cause of the first step that did not pass, then run the audit again with a new output directory.
:::

See [Read the Results](../03-read-the-results.md#statuses-of-audit-tally) for all statuses.

**Expected result:** you know the status of each scope, of each election history and of the Braid
verifier.

## 4. Record the digest of the audit record

1. Calculate the digest of the audit record:

   ```sh
   shasum -a 512 ~/audit/results/tally/audit-record.json
   ```

2. Compare it with the digest that the command showed.
3. Write the digest in the audit notes.
4. Ask a second person to make sure that the digest in the audit notes is the digest of the file.
   Then ask the person to sign the audit notes.

**Expected result:** the audit notes have the digest of the audit record and a second signature.

## If there is a problem

| Problem | Action |
| --- | --- |
| Exit status `4`, `output directory already exists` | Use a new output directory. Do not delete evidence that you need. |
| Exit status `3`, `expected exactly one tally session row, found none` | The tally ID is wrong, or the backup is from before the tally. Check the tally ID with the election authority. |
| `no copies in ...; run audit-env up first` | Restore the copies. See [Restore the Database Copies](02-restore.md). |
| Exit status `4`, `cannot start ...` in the `message` of a step | The image is not complete. Build it again. See [Set Up the Audit Workstation](01-workstation.md#2-build-the-audit-image). |
| You must repeat or examine one step. | Use [Run Each Step Separately](../07-run-each-step/00-run-each-step.md). |

See [Troubleshooting](../04-troubleshooting.md#messages-of-audit-tally) for all messages.

## Next

[Report and Remove the Copies](04-close.md)
