---
id: ballot_audit_manual
slug: /election_auditors/ballot_audit_manual
title: Ballot Audit Manual
sidebar_position: 0
description: "This manual tells election auditors how to check, from restored copies of the election data, that the ballots in a tally are the ballots that the voters cast."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

import ManualPdfLink from '@site/src/components/ManualPdfLink';

# Ballot Audit Manual

This manual tells election auditors how to check that the ballots in a tally are the ballots
that the voters cast. You do the audit offline, on restored copies of the election data. You do
not need the election application or access to the production systems.

<ManualPdfLink manual="ballot-audit" />

## What the audit checks

The audit connects four records of each counted ballot:

1. The signed record in the electoral log that the voter cast the ballot.
2. The stored ballot and its receipt.
3. The list of voters that can vote in the area, and the rule that keeps the last valid vote of
   each voter.
4. The encrypted ballots that went into the tally on the bulletin board.

A separate check with the Braid verifier examines the cryptography of the mix and of the
decryption. You must do both checks for a complete audit.

## How to use this manual

1. Read [Before You Start](01-before-you-start.md) first. It gives the roles, the terms and the
   safety rules.
2. Do the procedures in the order of the table. Each procedure tells you what you must have
   before you start.
3. Use [Read the Results](03-read-the-results.md) to understand each result.
4. Use [Troubleshooting](04-troubleshooting.md) when a command stops with a message.
5. Use [Command Reference](05-command-reference.md) for all options of the commands and the
   format of the input files.
6. Use [File Reference](06-file-reference.md) for the content of the files that the commands
   write.

## Procedures

| Step | Procedure | Content | Result |
| --- | --- | --- | --- |
|  | [Before You Start](01-before-you-start.md) | Roles, terms and safety rules | You know who does each task. |
| 1 | [Set Up the Audit Workstation](02-procedures/01-workstation.md) | An isolated computer and the tools | The tools run on the workstation. |
| 2 | [Restore the Database Copies](02-procedures/02-restore.md) | The copies of the four databases | The copies run on the workstation, with read-only accounts. |
| 3 | [Get the Scope and the Pins](02-procedures/03-scope.md) | The tally to audit and the trusted keys | You have a pinned `scope.json` file. |
| 4 | [Export the Evidence](02-procedures/04-export.md) | Files from the copies | The evidence files and their digests. |
| 5 | [Prepare the Audit Bundle](02-procedures/05-prepare.md) | The selection of the counted ballots | An audit bundle. |
| 6 | [Audit the Bundle](02-procedures/06-audit.md) | The independent check and receipt lookups | An audit report. |
| 7 | [Verify the Bulletin Board](02-procedures/07-braid.md) | The mix and decryption proofs | A Braid result for each batch. |
| 8 | [Check the Cast-Vote History](02-procedures/08-history.md) | Optional: all areas against the electoral log | A history report. |
| 9 | [Report and Remove the Copies](02-procedures/09-close.md) | The audit record and the clean-up | A published summary. No copies are left. |

## Scope of this manual

This manual is for version **10.0** of the ballot audit tools. It covers one tally, one area and
one contest in each audit run. It does not cover the final count of the results, the history
proofs of the electoral log database, or a link between a voter and a decrypted vote.
