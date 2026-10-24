---
title: Run the Audit
sidebar_position: 3
description: "Audit all areas and contests of one tally with one docker run command, read the results and record the digest of the audit record."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Run the Audit

This procedure audits all areas and contests of one tally with one command. The auditor does it.
The command runs the audit image with the backups directory and the output directory.

The command does these steps:

1. It checks the digest of each backup. It stops if a digest is missing or different.
2. It restores the backups into copies inside the container, with the read-only account
   `ballot_audit_reader`. It gives the ImmuDB copy a new admin password, so you do not need the
   password of the source.
3. It starts the B3 server on the B3 copy.
4. It runs `audit-tally`. `audit-tally` makes one scope for each area and contest of the tally,
   exports the evidence, prepares and audits each scope, checks the cast-vote history of each
   election and runs the Braid verifier.
5. It writes the results to the output directory and stops. The copies are removed.

The container has no network. The copies can connect only to each other.

`audit-tally` calculates the pin of each file that it writes and gives it to the next step. It
records all pins in `audit-record.json`. The digest of `audit-record.json` protects all of them.

## Before you start

- You did [Prepare the Backups](02-restore.md).
- `AUDIT_IMAGE` is set. See [Set Up the Audit Workstation](01-workstation.md).
- You have these three values from the election authority, through a trusted channel that is not
  the backups:
  - The **tally ID**.
  - The **public key hash**: the SHA-512 digest of the election public key on the bulletin board,
    in 128 lowercase hexadecimal characters.
  - The **electoral log system public key**: DER/SPKI, encoded in standard base64.
- The directory `~/audit/output` is empty.

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
   docker run --rm --network none \
     -v ~/audit/backups:/backups:ro -v ~/audit/output:/output "$AUDIT_IMAGE" \
     --tally-session-id "$TALLY_SESSION_ID" \
     --public-key-sha512 "$PUBLIC_KEY_SHA512" \
     --electoral-log-system-public-key "$ELECTORAL_LOG_SYSTEM_PUBLIC_KEY"
   ```

:::note
The command runs the steps one after the other. A large tally can take a long time.
:::

**Expected result:** the command shows `SHA-512 matches` for each backup, `the copies are
ready`, one line for each scope, the overall status, the digest of the audit record and, in the
last line, the exit status:

```text
scopes/001: election <ID> area <ID> contest <ID>: Success
scopes/002: election <ID> area <ID> contest all: Success
Overall: Success
audit-record.json SHA-512: <digest>
ballot-audit: exit status 0 (all steps passed)
```

`contest all` is a scope whose ballots encrypt all contests together.

## 3. Read the results

1. If the command shows `WARNING: pg_restore reported ... error(s)`, read the log that the
   warning names in `~/audit/output/restore`. Write each error in the audit notes. The command
   then stops with `not auditing: the restore of ... had errors` and exit status `4`.
2. Read the exit status in the last line of the command:

   | Exit status | Meaning |
   | --- | --- |
   | `0` | All steps passed. |
   | `1` | A step found a contradiction. Read the message. |
   | `2` | An option of the command is wrong. |
   | `3` | Evidence is missing, or a step did not run. The audit is incomplete. |
   | `4` | An input was refused. A refused backup or a restore with errors stops the command before the audit. A step of the audit can also return `4`: read the result files. |

3. Open `~/audit/output/public-summary.json`. Read `status`, the `status` of each scope, of each
   `cast_vote_history` entry and of `braid`.
4. For each status that is not `success`, open `~/audit/output/audit-record.json`. Find the steps
   of that scope or election in `steps`. Read their `exit_code` and `message`.
5. Find each message in [Troubleshooting](../04-troubleshooting.md).
6. Write the overall status and the status of each scope in the audit notes.

:::note
A step with the status `not_run` did not run because an earlier step did not pass. Correct the
cause of the first step that did not pass, then run the audit again with a new, empty output
directory.
:::

See [Read the Results](../03-read-the-results.md#statuses-of-audit-tally) for all statuses.

**Expected result:** you know the status of each scope, of each election history and of the Braid
verifier.

## 4. Record the digest of the audit record

1. Calculate the digest of the audit record:

   ```sh
   shasum -a 512 ~/audit/output/audit-record.json
   ```

2. Compare it with the digest that the command showed.
3. Write the digest in the audit notes.
4. Ask a second person to make sure that the digest in the audit notes is the digest of the file.
   Then ask the person to sign the audit notes.

**Expected result:** the audit notes have the digest of the audit record and a second signature.

## If there is a problem

| Problem | Action |
| --- | --- |
| `backup digest mismatch: backups/...; do not restore it` | Stop. Do not use the backup. Tell the election authority. |
| `the output directory is not empty; use a new one` | Use a new, empty output directory. Do not delete evidence that you need. |
| `missing backup: backups/...` or `no digest for ... in backup-digests.txt` | Put the backup in `~/audit/backups` with its file name, and its digest in the digests file. See [Prepare the Backups](02-restore.md). |
| `... did not start` | The command shows the end of the log. Read the logs in `~/audit/output/restore`. |
| `not auditing: the restore of ... had errors` | Write the errors in the audit notes and ask the system operator for a backup that restores without errors. See [Troubleshooting](../04-troubleshooting.md#messages-of-the-audit-image). |
| Exit status `3`, `expected exactly one tally session row, found none` | The tally ID is wrong, or the backup is from before the tally. Check the tally ID with the election authority. |
| You must repeat or examine one step. | Use [Run Each Step Separately](../07-run-each-step/00-run-each-step.md). |

See [Troubleshooting](../04-troubleshooting.md#messages-of-the-audit-image) for all messages.

## Next

[Report and Remove the Copies](04-close.md)
