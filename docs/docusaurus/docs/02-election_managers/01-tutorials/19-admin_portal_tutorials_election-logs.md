---
id: admin_portal_tutorials_election-logs
title: Election Logs
description: "The procedures Publish and Manage the Voting Period, Run the Tally Ceremony and Get the Results give the main path of an election."
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The procedures [Publish and Manage the Voting Period](../03-procedures/05-publish.md), [Run the Tally Ceremony](../03-procedures/06-tally.md)
and [Get the Results](../03-procedures/07-results.md) give the main path of an election. Each important
action of that path makes a record in the electoral log. This page tells you how to read, filter,
export and audit the electoral log.

## What the electoral log is

Each election event has its own electoral log. The platform adds a record for each important
action, and signs each record. You cannot change or delete a record from the admin portal. The
electoral log is the evidence of what occurred in the election event and when.

The electoral log records, for example:

- The publication of the ballot, and publications that failed.
- Each opening, pause and closing of a voting channel, for an election or for the election event.
- The key ceremony, and the upload of each key fragment in a tally ceremony.
- The start and the end of each tally.
- Each pause of a tally for a tie, each tie resolution and each restart.
- Each vote cast, with its channel, and each error when a voter casts a vote.
- The publication and the revocation of results on the results website.
- Messages sent to voters, events of the voter accounts, and changes to certificate authorities.

## Open the Logs tab

1. Open the election event.
2. Click the **Logs** tab.

The tab shows the newest records first. To see the tab, your account needs
**View Election Event Logs**. To read the records, it needs **Read Logs**. Without it, the tab
shows "You don't have permission to access logs."

## The columns

| Column | Content |
| --- | --- |
| **Id** | The number of the record in the log. |
| **User Id** | The id of the user that did the action. Hidden by default. |
| **Username** | The name of the user that did the action, or "-" for an action of the system. |
| **Created** | The date and time when the platform stored the record. |
| **Statement Timestamp** | The date and time of the action. |
| **Statement kind** | The kind of record, for example `ElectionPublish`, `ElectionVotingPeriodOpen`, `TallyOpen` or `CastVote`. |
| **Statement kind** (second column) | `USER` for an action of a person, `SYSTEM` for an action of the platform. |
| **Log Type** | `INFO` or `ERROR`. |
| **Description** | A short text that tells what occurred. Click it to see all the text. |
| **Message** | The full content of the record. |

To show or hide columns, use the columns button. Your account needs
**Election Event Logs Columns** for this button.

## Filter the records

Click the filter button and select a filter:

- **User Id** or **Username**: the records of one user.
- **Created** or **Statement Timestamp**: the records of one date and time.
- **Statement kind**: the records of one kind, for example `ElectionVotingPeriodClose`.

## Export the log

Your account needs **Export Logs**.

1. On the **Logs** tab, click the export button.
2. Click **Export in CSV** or **Export in PDF**.
3. Read the message "Please confirm you want to execute this action, it might take a while to
   execute."
4. Click **Export**.

**Expected result:** a task **Export Activity Logs Report** shows in the task panel. When it is
complete, download the file.

The export contains the activity log of the election event. For a regular export, use a report of
the type **Activity Logs** on the **Reports** tab. You can make it **Repeatable** with a
**Cron Expression** and **Email Recipients**. See [Reports and templates](18-reports_and_templates.md).

## Audit the log

The audit checks each record against the Merkle log of the election event, and verifies the
published checkpoints. A Merkle log is a chain of hashes: a change to one record breaks the chain.
Your account needs **Audit Electoral Log**.

1. On the **Logs** tab, click **Audit**.
2. Read the message in **Audit electoral log**.
3. Click **Audit**.

**Expected result:** a task **Audit Electoral Log** shows in the task panel. The result of the
audit is recorded in the task. You can also see it on the **Tasks** tab of the election event.

## The logs of one voter

1. Open the election event.
2. Click the **Voters** tab.
3. In the row of the voter, open the actions menu.
4. Click **User's Logs**.

The dialog shows the records of the electoral log for that voter only. Your account needs
**View Election Event Voters Logs**.

## The Electoral Log page

The main menu has the item **Electoral Log**. This page lets an auditor browse the electoral log
and the ballot box of an election event, or query the electoral log database. The page does not
change any data. Your account needs **Browse Electoral Log**. Usernames, IP addresses and countries
show as hidden without **Read Electoral Log Personal Data**.

## Why the logs matter for an audit

- Use the log to prove when each channel opened and closed, and who did it.
- Compare the records of the tally with the records of the tally ceremony.
- Run the audit before you publish the results, and keep the task result.
- Export the log at the end of the election, and keep it with the other records. See
  [Get the Results](../03-procedures/07-results.md#5-keep-the-records).

## If there is a problem

| Problem | Action |
| --- | --- |
| The **Logs** tab is not shown. | Ask for the permission **View Election Event Logs**. |
| "You don't have permission to access logs." | Ask for the permission **Read Logs**. |
| The export button or **Audit** is not shown. | Ask for the permission **Export Logs** or **Audit Electoral Log**. |
| The export or audit task failed. | Look at the **Tasks** tab. Try again, then contact Sequent support. |
| The audit finds an error. | Do not publish the results. Contact Sequent support immediately and give the name of the election event. |
| A record is not in the list. | Remove the filters. The list shows the newest records first. |
