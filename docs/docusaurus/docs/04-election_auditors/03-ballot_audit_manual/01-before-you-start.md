---
title: Before You Start
sidebar_position: 1
description: "Read this page before you do a procedure. It gives the order of the work, the roles, the terms, the conventions and the rules that keep the audit safe."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Before You Start

Read this page before you do a procedure. It gives the order of the work, the roles, the terms,
the versions, the conventions and the rules that keep the audit safe.

## The order of the work

1. [Set Up the Audit Workstation](02-procedures/01-workstation.md)
2. [Restore the Database Copies](02-procedures/02-restore.md)
3. [Run the Audit](02-procedures/03-run-the-audit.md)
4. [Report and Remove the Copies](02-procedures/04-close.md)

Do the procedures in this order. If the audit does not pass, read
[Read the Results](03-read-the-results.md) before you report. To repeat or examine one step, use
[Appendix: Run Each Step Separately](07-run-each-step/00-run-each-step.md).

## Roles

| Role | Tasks |
| --- | --- |
| Election authority | Decides which tally to audit. Gives the auditor the tally ID, the trusted keys and the backups. Receives the audit report. |
| System operator | Makes the backups of the four databases. Records the identity and the digest of each backup. |
| Auditor | Restores the copies, runs the tools and writes the audit report. |
| Second person | Checks the digests in the audit notes and signs them. Must not be the auditor. |

The auditor must not be the person who made the backups. If one person has both roles, write
this in the audit report.

## Terms

| Term | Meaning |
| --- | --- |
| Tally | One count of the votes. An election event can have more than one tally. |
| Scope | One election, one area and one contest of a tally. `audit-tally` makes one scope for each area and contest of the tally. |
| Electoral log | The signed log of the election events, stored in ImmuDB. A `CastVote` record contains signed evidence of a cast ballot. A missing record makes the audit incomplete. |
| Bulletin board | The B3 database of the cryptographic protocol. It holds the election public key and the encrypted ballots of each tally. |
| Batch | A numbered group of encrypted ballots on the bulletin board. A tally of one area and one contest uses one or more batches. |
| Receipt | The ballot ID that the voter gets after the vote. It is a hash of the encrypted ballot. |
| Revote | A second or later vote of the same voter. Only the last valid vote counts. |
| Pin | A SHA-512 digest of a file that you get from a trusted source. The tools refuse a file that does not match its pin. |
| Digest | The SHA-512 hash of a file: 128 lowercase hexadecimal characters. |
| Evidence | The files that the export procedures write. |
| Audit bundle | The files that **Prepare the Audit Bundle** writes. They contain only the ballots of the scope. |
| Exit status | The number that a command returns when it stops. It tells the result. See [Read the Results](03-read-the-results.md). |
| Audit record | The file `audit-record.json` that `audit-tally` writes. It records each command, its result and the digests of its files. |
| Audit notes | The text file `audit-notes.txt` in which you write the values that the procedures tell you to record. |
| Audit image | The Docker image with all tools of the audit and the database servers of the copies. One `docker run` command restores the backups into copies inside the container, runs the audit and removes the copies. |

## Versions

This manual and the [technical explanation](../04-ballot_audit_explanation.md) describe proposed tools in
[PR #3968](https://github.com/sequentech/step/pull/3968), reviewed at `820352256b8d`.
The code and [manual PR #4048](https://github.com/sequentech/step/pull/4048) are drafts at the time of writing.
This documentation does not establish release, deployment or availability of a published image.

Before reporting verified decryption, use a verifier revision that requires successful plaintext recomputation for every batch.
The reviewed revision does not enforce this requirement in its success condition.
See [Check the cryptographic tally](../04-ballot_audit_explanation.md#6-check-the-cryptographic-tally) for the checks and their interpretation.

| Item | Version |
| --- | --- |
| Election data | The release of the election platform that ran the election, for example `10.0.1-rc.1`. |
| Tools | The audit image, built from a commit of the same release line that contains the folder `packages/ballot-audit/image`. A release from before the tools does not contain this folder. |
| Database servers | The audit image contains PostgreSQL 18 and ImmuDB 1.9.6. It restores backups of PostgreSQL 18 and earlier, and the ImmuDB data directory of the platform. |

Write the release of the data, the commit of the tools and the ID of the audit image in the audit
notes.

Record the actual source commit and image identity separately.
The current output field `source_revision` names the baseline `d6d05a1`, not the executable's full commit.
The image's reported tests cover Linux amd64. They do not establish support for arm64 or Docker Desktop on macOS.

Earlier [synthetic tally tests](https://github.com/sequentech/meta/issues/14374#issuecomment-6365862832) include successful Braid checks on actually mixed tallies at `1fec078701b`.
Those tests used individual commands on a development stack with test-specific configuration changes.
The [restored-copy tests](https://github.com/sequentech/meta/issues/14374#issuecomment-6366819273) used manual restoration.
Both predate `audit-tally` and the audit image.
The current image's reported run used a never-mixed synthetic board, so real Braid verification failed as expected.
A complete successful image run on an actually mixed tally remains to be demonstrated.

Some orchestrator tests use a stand-in verifier to test command handling.
These tests and the reported runs do not establish acceptance for a production election.

## Conventions

- `Monospace text` is a command, a file name or a value that you type.
- A line that starts with `export` sets a shell variable. Later commands use the variable, for
  example `$SCOPE_SHA512`.
- The commands are for a Linux or macOS shell.
- A procedure has these parts: **Before you start**, numbered steps, **Expected result**,
  **If there is a problem** and **Next**.
- After each command, read the exit status with `echo $?`.

## Notices

| Notice | Meaning |
| --- | --- |
| **WARNING** | Risk to the secrecy of the votes, to the personal data of the voters or to the result of the audit. |
| **CAUTION** | Risk of the loss of evidence, or of an action that you cannot reverse. |
| **NOTE** | Useful information that is not a risk. |

## Safety rules

:::danger WARNING
Do not connect the tools to a production system. Use only restored copies on an isolated
workstation. The audit image runs the copies and the tools in a container without a network.
The tools also refuse remote connections, but without the image a tunnel or a proxy can go
around this protection.
:::

:::danger WARNING
Keep the copies, the evidence and the audit bundle private. They contain voter IDs, receipts and
ballots. Publish only the summary that [Report and Remove the Copies](02-procedures/04-close.md)
describes.
:::

:::danger WARNING
Get the trusted keys and the pins from outside the files that you audit. If a person can change
both the files and their pins, the audit proves nothing.
:::

:::caution CAUTION
Do not change the original backups. Work on copies. If you change a backup, you lose the
evidence.
:::

## What the audit result means {#what-the-audit-proves}

At the reviewed implementation, `audit-tally` returns `0` when all recorded steps report success.
The checks connect the selected ballots to signed cast records, supplied eligibility and encrypted tally inputs.
Braid checks the public key, mix evidence and references in the published plaintext records.
Its decryption routine checks proofs and recomputes plaintexts when it runs, but the current success condition does not require its completion.
The result depends on the trusted inputs, supported scope and available evidence.

Read the restore logs and each report before reporting a pass.
The image can continue after restore errors, and some saved annotation checks can be unavailable without failing the command.
See [What a successful run establishes](../04-ballot_audit_explanation.md#what-a-successful-run-establishes) for the connection between the checks.

The audit does not prove these points:

- That the backups are complete, or that the four backups are from the same moment.
- The history of the electoral log database. The audit checks the signatures of the records,
  not the inclusion proofs of the database.
- That a voter really voted. The audit cannot link a voter to a decrypted vote.
- The final count of the results.
- The validity of each ballot's proof of plaintext knowledge or any voter ballot signature; this workflow does not re-verify them.
- The authorization of eligibility, weight, delegation or discard decisions. The supplied records must preserve the intended tally-time state.

Write these limits in the audit report.

The image checks the backups against `backup-digests.txt` in the same directory.
Authenticate that file separately; a match alone does not establish the backups' origin.
The audit record's restore references contain the three PostgreSQL dump digests.
Record the ImmuDB and any blob archive digests separately in the audit notes.

History reconciliation assumes the Hasura backup includes every cast recorded in the ImmuDB backup.
A later ImmuDB backup can otherwise produce a contradiction without tampering.
The source acquisition uses unsigned database metadata to find rows, so capture completeness still needs independent evidence.

The exporter supports default key ceremonies and refuses combined voter-weighted and delegated voting.
History verification uses one pinned system signing key; rotated-key records require separate handling.
