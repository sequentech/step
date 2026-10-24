---
title: Report and Remove the Copies
sidebar_position: 4
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

- You did [Run the Audit](03-run-the-audit.md).
- The audit notes contain all values that the procedures told you to record, with a second
  signature.

## 1. Write the audit report

1. Write the tally ID, the public key hash and the electoral log system public key.
2. Write the source commit of the tools.
3. Write the identity and the digest of each backup.
4. Write the digest of `audit-record.json`.
5. Write the overall status, and the status and the counts of each scope, of each election history
   and of the Braid verifier. Copy them from `public-summary.json`.
6. For each status that is not `success`, write the message of the step and your conclusion.
7. If you ran a step separately, write its pins, its exit status and its counts.
8. Write the limits of the audit. See [Before You Start](../01-before-you-start.md#what-the-audit-proves).

**Expected result:** the audit report is complete.

## 2. Publish the safe summary

1. Publish `~/audit/output/public-summary.json`. It contains only IDs of the election,
   statuses and counts. See [File Reference](../06-file-reference.md#public-summaryjson).
2. Publish the digest of `audit-record.json`, the commit of the tools and the limits of the audit.
3. Do not publish other information. Use this table:

| Safe to publish | Do not publish |
| --- | --- |
| `public-summary.json`. | Voter IDs and hashed voter identifiers. |
| The digests of the backups and of `audit-record.json`. | Ballot IDs and receipts that identify a voter. |
| The commit of the tools. | Batch positions in a batch that `privacy_warnings` lists. |
| The limits of the audit. | The copies, the evidence, the audit bundles and the reports. |

:::danger WARNING
Do not publish the files in `output/scopes/`, `output/history/` or `output/immudb-export/`.
They contain personal data, for example `selection.json`, `cast-votes.json`,
`cast-vote-history.json`, `keycloak-source.json`, `eligibility-reasons.json` and
`audit-report.json`.
:::

**Expected result:** the election authority has the audit report. The public summary contains no
personal data.

## 3. Keep the evidence

1. Ask the election authority how long to keep the evidence.
2. Put the directory `~/audit/output` and the audit notes in an encrypted archive.
3. Give the archive to the election authority.

**Expected result:** the election authority keeps the evidence.

## 4. Remove the copies

:::caution CAUTION
Do this step only after the election authority has the archive. You cannot get the copies back.
:::

The copies were inside the container of the audit. Docker removed them when the command
stopped.

1. Remove the audit directory, which contains the backups and the output:

   ```sh
   rm -rf ~/audit
   ```

2. Write the date of the removal in the audit report.

:::note
If you used [Without the Audit Image](../07-run-each-step/01-without-kit.md), also remove the
containers, the volume and the network that you made there, with `docker rm -f`,
`docker volume rm` and `docker network rm`.
:::

**Expected result:** no copy of the election data is on the workstation.

## Next

The audit is complete.
