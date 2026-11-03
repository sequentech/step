---
id: election_management_election_event_logs
title: Logs
description: "The Logs tab shows the record of the actions in the election event, for example the ceremonies, the publications and the changes of the voting status. Use it to check what happened and when."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Logs** tab shows the record of the actions in the election event, for example the
ceremonies, the publications and the changes of the voting status. Use it to check what
happened and when. You can export the log for your records.

## Open the tab

1. In the menu on the left, click the election event.
2. Click the **Logs** tab.

The tab shows with the **View Election Event Logs** permission. To read the entries, your role
needs **Read Logs**. Without it, the tab shows "You don't have permission to access logs.".
The tab stays visible when the election event is locked down.

## Toolbar

| Button | Use | Permission |
| --- | --- | --- |
| **Columns** | Selects the columns. | **Election Event Logs Columns** |
| **Add filter** | Filters by **User Id**, **Username**, **Created**, **Statement Timestamp** or **Statement kind**. | **Election Event Logs Filters** |
| **Export** | Opens a menu with **Export in CSV** and **Export in PDF**. | **Export Logs** |

## Columns

| Column | Content |
| --- | --- |
| **Id** | The number of the entry. |
| **User Id** and **Username** | The user who did the action. |
| **Created** | The date and time of the entry. |
| **Statement Timestamp** | The date and time of the action. |
| **Statement kind** | The kind of action. |
| **Log Type** | The type of entry. |
| **Description** | A description of the action. |
| **Message** | The full content of the entry. |

## Export the log

1. Click **Export**.
2. Click **Export in CSV** or **Export in PDF**.
3. Read the message "Please confirm you want to execute this action, it might take a while to
   execute."
4. Click **Export**.
5. When the task "Export Activity Logs Report" is complete, click **Download File**.

**Expected result:** you have the log file.

## If there is a problem

| Problem | Action |
| --- | --- |
| "You don't have permission to access logs." | Your role does not have the **Read Logs** permission. Ask your tenant administrator. |
| The export takes a long time. | A large log takes time. Wait for the task to finish. |

## Related pages

- [Tasks](../Tasks/election_management_election-event_tasks.md)
- [Voters](../Voters/election_management_election-event_voters.md) (the **User's Logs** action shows the entries of one voter)
- [Reports](../Reports/election_management_election-event_reports.md)
