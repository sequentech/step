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
the conventions and the rules that keep the audit safe.

## The order of the work

1. [Set Up the Audit Workstation](02-procedures/01-workstation.md)
2. [Restore the Database Copies](02-procedures/02-restore.md)
3. [Get the Scope and the Pins](02-procedures/03-scope.md)
4. [Export the Evidence](02-procedures/04-export.md)
5. [Prepare the Audit Bundle](02-procedures/05-prepare.md)
6. [Audit the Bundle](02-procedures/06-audit.md)
7. [Verify the Bulletin Board](02-procedures/07-braid.md)
8. [Check the Cast-Vote History](02-procedures/08-history.md)
9. [Report and Remove the Copies](02-procedures/09-close.md)

Do the procedures in this order. **Check the Cast-Vote History** is optional. If a command
stops with a failure, do not continue to the next procedure. Read
[Read the Results](03-read-the-results.md) first.

## Roles

| Role | Tasks |
| --- | --- |
| Election authority | Decides which tally to audit. Gives the auditor the trusted keys and the backups. Receives the audit report. |
| System operator | Makes the backups of the four databases. Records the identity and the digest of each backup. |
| Auditor | Restores the copies, runs the tools and writes the audit report. |

The auditor must not be the person who made the backups. If one person has both roles, write
this in the audit report.

## Terms

| Term | Meaning |
| --- | --- |
| Tally | One count of the votes. An election event can have more than one tally. |
| Scope | The part of a tally that one audit run covers: one tally, one election, one area and one contest. |
| Electoral log | The signed log of the election events. It is stored in an ImmuDB database. Each cast vote has a signed `CastVote` record. |
| Bulletin board | The B3 database of the cryptographic protocol. It holds the election public key and the encrypted ballots of each tally. |
| Batch | A numbered group of encrypted ballots on the bulletin board. A tally of one area and one contest uses one or more batches. |
| Receipt | The ballot ID that the voter gets after the vote. It is a hash of the encrypted ballot. |
| Revote | A second or later vote of the same voter. Only the last valid vote counts. |
| Pin | A SHA-512 digest of a file that you get from a trusted source. The tools refuse a file that does not match its pin. |
| Digest | The SHA-512 hash of a file: 128 lowercase hexadecimal characters. |
| Evidence | The files that the export procedures write. |
| Audit bundle | The files that **Prepare the Audit Bundle** writes. They contain only the ballots of the scope. |
| Exit status | The number that a command returns when it stops. It tells the result. See [Read the Results](03-read-the-results.md). |

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
workstation. The tools refuse remote connections, but a tunnel or a proxy can go around this
protection.
:::

:::danger WARNING
Keep the copies, the evidence and the audit bundle private. They contain voter IDs, receipts and
ballots. Publish only the summary that [Report and Remove the Copies](02-procedures/09-close.md)
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

## What the audit proves

If all commands return `0`, the selected ballots in the copies agree with the signed electoral
log, with the frozen list of voters, with the saved tally and with the encrypted ballots on the
bulletin board.

The audit does not prove these points:

- That the backups are complete, or that the four backups are from the same moment.
- The history of the electoral log database. The audit checks the signatures of the records,
  not the inclusion proofs of the database.
- That a voter really voted. The audit cannot link a voter to a decrypted vote.
- The final count of the results.

Write these limits in the audit report.
