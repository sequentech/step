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

This manual explains **how** to run the audit. For **what** each check establishes, read
[What the Ballot Audit Verifies](../04-ballot_audit_explanation.md).
That explanation follows the checks from signed cast records through encrypted inputs, verified mixes and published plaintext records.

## What the audit checks

The audit connects four records of each selected ballot:

1. The signed record in the electoral log that the voter cast the ballot.
2. The stored ballot and its receipt.
3. The supplied eligibility list and the rule that selects each voter's latest cast row marked valid.
4. The encrypted ballots that went into the tally on the bulletin board.

A separate check with Braid verifies the public key, the mix evidence and the decryption, which it recomputes for every batch.
See [Versions](01-before-you-start.md#versions) for the revisions that require that recomputation.
You must do both parts of this audit. The `audit-tally` command runs both
parts for the areas and contests in the supplied tally mappings.

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
7. Use [Appendix: Run Each Step Separately](07-run-each-step/00-run-each-step.md) to run one
   command at a time.

## Procedures

| Step | Procedure | Content | Result |
| --- | --- | --- | --- |
|  | [Before You Start](01-before-you-start.md) | Roles, terms, versions and safety rules | You know who does each task. |
| 1 | [Set Up the Audit Workstation](02-procedures/01-workstation.md) | Docker and the audit image | The audit image is on the workstation. |
| 2 | [Prepare the Backups](02-procedures/02-restore.md) | The backups of the four databases and their digests | One directory with the backups. |
| 3 | [Run the Audit](02-procedures/03-run-the-audit.md) | One `docker run` command for all areas and contests of the tally | An audit record and a public summary. |
| 4 | [Report and Remove the Copies](02-procedures/04-close.md) | The audit report and the clean-up | A published summary. No copies are left. |

To repeat or examine one step of the audit, or to work without the audit image, use
[Appendix: Run Each Step Separately](07-run-each-step/00-run-each-step.md).

## Scope of this manual

This manual describes the proposed ballot audit tools for the **10.0** release line. See
[Versions](01-before-you-start.md#versions). Each audit run covers one tally, with all its areas
and contests represented in the supplied mappings.
The manual does not cover the final count or the electoral log database's history proofs.
It does not establish a link between a voter and a decrypted vote.
