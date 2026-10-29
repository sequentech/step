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
5. It shows each step, the content of each report and the results on the console. It writes a
   copy of the console output to `audit-output.log` in the output directory, and the digest of
   that file to `audit-output.log.sha512`. Then it stops. The copies and all other files of the
   audit are removed.

The container has no network. The copies can connect only to each other.

`audit-tally` calculates the pin of each file that it writes and gives it to the next step. The
console output shows each pin. The digest of `audit-output.log` protects the console output.

The console output contains no voter IDs, no ballot contents and no Keycloak users. It contains
the cast vote IDs and the receipts of the stored votes, so keep `audit-output.log` private.

For a prepared client package, run this from its extracted directory instead of the raw-image setup and command below:

```sh
./run-audit.sh OUTPUT_DIRECTORY
```

The launcher already sets the tally ID, public-key hash, electoral-log system public key and environment slug.
Replace `OUTPUT_DIRECTORY` with a new directory path; its parent must exist, but the directory itself must not exist.

## Before you start

- You did [Prepare the Backups](02-restore.md).
- `AUDIT_IMAGE` is set. See [Set Up the Audit Workstation](01-workstation.md).
- You have these three values from the election authority, through a trusted channel that is not
  the backups:
  - The **tally ID**.
  - The **public key hash**: the SHA-512 digest of the election public key on the bulletin board,
    in 128 lowercase hexadecimal characters.
  - The **electoral log system public key**: DER/SPKI, encoded in standard base64.
  - The **environment slug**, only if the election uses its own keys ceremony: the short
    name of the deployment, for example `example`. The election authority gets it from the
    system operator.
- The directory `~/audit/output` is empty.

## 1. Set the trusted values

1. Set the three values from the election authority:

   ```sh
   export TALLY_SESSION_ID=<tally ID>
   export PUBLIC_KEY_SHA512=<public key hash>
   export ELECTORAL_LOG_SYSTEM_PUBLIC_KEY=<electoral log system public key>
   ```

   If you have an environment slug, set it too:

   ```sh
   export ENVIRONMENT_SLUG=<environment slug>
   ```

2. Write the values in the audit notes.

**Expected result:** the variables are set. The audit notes contain the values.

## 2. Run the audit

1. Run the audit:

   ```sh
   docker run --rm --network none \
     -v ~/audit/backups:/backups:ro -v ~/audit/output:/output "$AUDIT_IMAGE" \
     --tally-session-id "$TALLY_SESSION_ID" \
     --public-key-sha512 "$PUBLIC_KEY_SHA512" \
     --electoral-log-system-public-key "$ELECTORAL_LOG_SYSTEM_PUBLIC_KEY"
   ```

   If you have an environment slug, add `--environment-slug "$ENVIRONMENT_SLUG"` at the end
   of the command.

:::note
The command runs the steps one after the other. A large tally can take a long time.
:::

**Expected result:** the command shows `SHA-512 matches` for each backup, the log of each
restore, `the copies are ready`, then one section for each step of the audit and a `Results`
section. The end of the output is:

```text
scopes/001: election <ID> area <ID> contest <ID>: Success
scopes/002: election <ID> area <ID> contest all: Success
Cast-vote history of election <ID>: Success (outcome consistent)
Overall: Success
Audit record SHA-512: <digest>
...
ballot-audit: exit status 0 (all steps passed)
ballot-audit: audit-output.log SHA-512: <digest>
```

The output directory contains only `audit-output.log` and `audit-output.log.sha512`. The last line
is not in `audit-output.log`: it is the digest of that file. The image does not keep the
audit record; `Audit record SHA-512` is in the log as a record of the run.

`contest all` is a scope whose ballots encrypt all contests together.

## 3. Read the results

1. If the command shows `WARNING: pg_restore reported ... error(s)`, read the log of
   `pg_restore` above the warning. Write each error in the audit notes. The command then stops
   with `not auditing: the restore of ... had errors` and exit status `4`.
2. Read the exit status in the last line of the command:

   | Exit status | Meaning |
   | --- | --- |
   | `0` | All steps passed. |
   | `1` | A step found a contradiction. Read the message. |
   | `2` | An option of the command is wrong. |
   | `3` | Evidence is missing, or a step did not run. The audit is incomplete. |
   | `4` | An input was refused. A refused backup or a restore with errors stops the command before the audit. A step of the audit can also return `4`: read the result files. |

3. Open `~/audit/output/audit-output.log` and go to the `Results` section. In
   `Public summary`, read `status`, the `status` of each scope, of each `cast_vote_history`
   entry and of `braid`, and `tallied_votes_to_audit`.
4. For each status that is not `success`, find the `Step` sections of that scope or election in
   `audit-output.log`. Read their `Status`, `Exit code` and `Messages`, and the reports below
   them.
5. Find each message in [Troubleshooting](../04-troubleshooting.md).
6. Write the overall status and the status of each scope in the audit notes.
7. If `tallied_votes_to_audit` is not `0`, the exit status line ends with
   `tallied votes to audit, listed above`. Review each vote in `Tallied votes to audit`, in the
   `Results` section of `audit-output.log`. See
   [Votes to audit](../03-read-the-results.md#votes-to-audit). The audit is not finished until
   you write the result of each vote in the audit report.

:::note
A step with the status `not_run` did not run because an earlier step did not pass. Correct the
cause of the first step that did not pass, then run the audit again with a new, empty output
directory.
:::

See [Read the Results](../03-read-the-results.md#statuses-of-audit-tally) for all statuses.

**Expected result:** you know the status of each scope, of each election history and of the Braid
verifier.

## 4. Record the digest of the audit output

1. Check the digest of the audit output:

   ```sh
   cd ~/audit/output && shasum -a 512 -c audit-output.log.sha512
   ```

   The command shows `audit-output.log: OK`.
2. Compare the digest in `audit-output.log.sha512` with the digest in the last line of the
   command.
3. Write the digest in the audit notes.
4. Ask a second person to make sure that the digest in the audit notes is the digest of the file.
   Then ask the person to sign the audit notes.

**Expected result:** the audit notes have the digest of the audit output and a second signature.

## If there is a problem

| Problem | Action |
| --- | --- |
| `backup digest mismatch: backups/...; do not restore it` | Stop. Do not use the backup. Tell the election authority. |
| `the output directory is not empty; use a new one` | Use a new, empty output directory. Do not delete evidence that you need. |
| `missing backup: backups/...` or `no digest for ... in backup-digests.txt` | Put the backup in `~/audit/backups` with its file name, and its digest in the digests file. See [Prepare the Backups](02-restore.md). |
| `... did not start` | The command shows the log of the copy that did not start. Read it in `audit-output.log`. |
| `not auditing: the restore of ... had errors` | Write the errors in the audit notes and ask the system operator for a backup that restores without errors. See [Troubleshooting](../04-troubleshooting.md#messages-of-the-audit-image). |
| Exit status `3`, `expected exactly one tally session row, found none` | The tally ID is wrong, or the backup is from before the tally. Check the tally ID with the election authority. |
| You must repeat or examine one step. | Use [Run Each Step Separately](../07-run-each-step/00-run-each-step.md). |

See [Troubleshooting](../04-troubleshooting.md#messages-of-the-audit-image) for all messages.

## Next

[Report and Remove the Copies](04-close.md)
