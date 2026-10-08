---
id: election_management_election_event_tasks
title: Tasks
description: "The Tasks tab shows the background tasks of the election event, for example imports, exports and reports. Use it to see if a task is complete, to read its logs and to download its file."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Tasks** tab shows the background tasks of the election event, for example imports,
exports and reports. Use it to see if a task is complete, to read its logs and to download its
file.

## Open the tab

1. In the menu on the left, click the election event.
2. Click the **Tasks** tab.

The tab shows with the **View Election Event Tasks** permission and **Read Tasks Execution**.
It is hidden when the election event is locked down.

When you start a long action, a task panel also shows at the bottom of the screen. It shows
"Task: ..." with the name of the task, its logs, **View Task** and, when there is a file,
**Download File**.

## Toolbar

| Button | Use | Permission |
| --- | --- | --- |
| **Columns** | Selects the columns. | **Election Event Tasks Columns** |
| **Add filter** | Filters by **Index**, **Type** or **Status**. | **Election Event Tasks Filters** |
| **Export** | Exports the list of tasks. | **Export Tasks** |

## Columns

| Column | Content |
| --- | --- |
| **Name** | The name of the task, for example **Import Users**. |
| **Start time** | The date and time of the start. |
| **Status** | `STARTED`, `IN_PROGRESS`, `SUCCESS`, `FAILED` or `CANCELLED`. |
| **Actions** | The view icon. It opens the details of the task. |

## Task details

| Element | Content |
| --- | --- |
| **Task Information** | The **Type**, the **Executer**, the **Start time** and the **End time**. |
| Logs | The messages of the task. Read them when the status is `FAILED`. |
| **Back** | Returns to the list. It needs the **Back to Election Event Tasks** permission. |
| **Download File** | Downloads the file of the task. It is available only when the status is `SUCCESS`. |

## Task types

| Task | Started by |
| --- | --- |
| **Create Election Event** | The creation of an election event. |
| **Import Election Event** | **Import Election Event** in the menu. |
| **Export Election Event** | **Export** on the **Data** tab. |
| **Import Users** | **Import** on the **Voters** tab. |
| **Export Voters** | **Export** on the **Voters** tab. |
| **Import Candidates** | **Import Candidates** on the **Data** tab. |
| **Export Ballot Publication** | **Export** on the **Changes to be Published** page. |
| **Generate Report** | **Generate** on the **Reports** tab. |
| **Export Activity Logs Report** | **Export** on the **Logs** tab. |
| **Export Applications** | **Export** on the **Approvals** tab. |
| **Render Document PDF** | The PDF option of an HTML result document. |

## If there is a problem

| Problem | Action |
| --- | --- |
| A task shows `FAILED`. | Open the task and read the logs. Correct the cause, then start the action again. |
| A task stays at `STARTED` or `IN_PROGRESS` for a long time. | Large imports and exports can take time. If nothing changes after a long time, contact Sequent support. |
| **Download File** is not available. | The task is not complete, or it does not make a file. |
| "Error exporting Tasks Execution" | Try the export again. |

## Related pages

- [Voters](../Voters/election_management_election-event_voters.md)
- [Data](../Data/election_management_election-event_data.md)
- [Reports](../Reports/election_management_election-event_reports.md)
- [Logs](../Logs/election_management_election-event_logs.md)
