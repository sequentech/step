---
title: Report and Remove the Copies
sidebar_position: 9
description: "Write the audit report, publish only the safe summary, and remove the restored copies and the private files."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Report and Remove the Copies

This procedure closes the audit. The auditor writes the audit report, gives it to the election
authority and removes the copies.

## Before you start

- You did all the procedures that the election authority asked for.
- The audit record contains all values that the procedures told you to record.

## 1. Write the audit report

1. Write the scope: the election event, the election, the area, the contest and the tally.
2. Write the source commit of the tools.
3. Write the identity and the digest of each backup.
4. Write the pins: the scope, the six evidence files, the manifest and the configuration.
5. For each command, write the exit status and the counts.
6. For each exit status that is not `0`, write the message and your conclusion.
7. Write the limits of the audit. See [Before You Start](../01-before-you-start.md#what-the-audit-proves).

**Expected result:** the audit report is complete.

## 2. Publish the safe summary

Publish only the information of the left column.

| Safe to publish | Do not publish |
| --- | --- |
| The exit statuses and the counts (`checked_*`). | Voter IDs and voter pseudonyms. |
| The digests of the backups, the evidence and the pins. | Ballot IDs and receipts that identify a voter. |
| The commit of the tools. | Batch positions in a batch that `privacy_warnings` lists. |
| The limits of the audit. | The copies, the evidence, the audit bundle and the reports. |

:::danger WARNING
Do not publish `selection.json`, `cast-votes.json`, `cast-vote-history.json`, the electoral log
export, `keycloak-source.json` or `eligibility-reasons.json`. They contain personal data.
:::

**Expected result:** the election authority has the audit report. The public summary contains no
personal data.

## 3. Keep the evidence

1. Ask the election authority how long to keep the evidence.
2. Put the evidence, the audit bundle, the reports and the audit record in an encrypted archive.
3. Give the archive to the election authority.

**Expected result:** the election authority keeps the evidence.

## 4. Remove the copies

:::caution CAUTION
Do this step only after the election authority has the archive. You cannot get the copies back.
:::

1. Stop the B3 server.
2. Remove the containers:

   ```sh
   docker rm -f audit-hasura-db audit-keycloak-db audit-b3-db audit-immudb
   ```

3. Remove the ImmuDB volume and the network:

   ```sh
   docker volume rm audit-immudb && docker network rm audit-net
   ```

4. Remove the audit directory, which contains the backups:

   ```sh
   rm -rf ~/audit
   ```

5. Write the date of the removal in the audit report.

**Expected result:** no copy of the election data is on the workstation.

## Next

The audit is complete.
