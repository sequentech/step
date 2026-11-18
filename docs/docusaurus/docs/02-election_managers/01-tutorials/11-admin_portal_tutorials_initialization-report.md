---
id: admin_portal_tutorials_initialization-report
title: Initialization Report
description: "The main path is in Publish and Manage the Voting Period. This page gives the details: when the report is necessary, what blocks it and how it unblocks the voting period."
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The main path is in [Publish and Manage the Voting Period](../03-procedures/05-publish.md#4-make-the-initialization-report-if-necessary).
This page gives the details: when the report is necessary, what blocks it and how it unblocks the
voting period.

## What the initialization report is

The initialization report is a tally of the ballot box before the voting period opens. It uses
the same tally ceremony as the count of the results. The ballot box is empty, so the report
shows zero votes. The report is evidence that the ballot box contained no votes at the start.

In the **Tally** list of the election event, the report shows with the **Tally Type**
**Initialization Results**. A count of the votes shows with the **Tally Type**
**Electoral Results**.

## When the report is necessary

Each election has the field **Initialize Report Policy** on its **Data** tab.

| Value | Effect |
| --- | --- |
| **Not Required** (default) | The report is optional. You can open the voting period without it. |
| **Required** | The election cannot open until an initialization report of the election is complete. |

When the policy is **Required**:

- On the **Publish** tab of the election, **Start Online Voting** is not available until the
  report is complete.
- The platform refuses to open any channel of the election from the **Publish** tab of the
  election, or with a scheduled event for the election.

:::caution CAUTION
Open the voting period of an election with this policy from the **Publish** tab of the election.
The **Publish** tab of the election event does not check the initialization report.
:::

## Permit the report at a set time

The scheduled event **Allow Initialization Report** sets the status of an election so that
initialization reports are permitted. By default, the status of an election permits them. If
the status of an election does not permit them, this event permits them at the set time. See [Scheduled events](13-admin_portal_tutorials_scheduled-events.md).

## Before you start

- The key ceremony status is `SUCCESS`. See [Run the Key Ceremony](../03-procedures/04-keys.md).
- The election event has a publication, and each election in the report is published. See
  [Publish and Manage the Voting Period](../03-procedures/05-publish.md).
- Your account can create tally ceremonies (**Create Ceremony** permission).
- For a manual key ceremony, the trustees are available with their key fragment.

If there is no successful key ceremony or no publication, **Generate Initialization Report** is
not available.

## Which elections you can select

On the **Elections for Initialization Report** page, **Start Initialization Report** stays
unavailable if one of the selected elections:

- Is not published.
- Already has an initialization report that is not cancelled.
- Already has a complete initialization report.
- Has the policy **Required** and a status that does not permit initialization reports.

The page then shows "You cannot continue the ceremony because no elections are selected or the
elections are not published." This message shows for all these reasons, not only for the two that
it names.

## Make the report

1. Open the election event.
2. Click the **Tally** tab.
3. Click **Generate Initialization Report**.
4. Select the elections.
5. In **Key Ceremony**, select the key ceremony of these elections.
6. Click **Start Initialization Report**.
7. Confirm the action.
8. Continue as in a tally ceremony: the trustees upload their key fragment, then you click
   **Start Tally**. See [Run the Tally Ceremony](../03-procedures/06-tally.md#2-upload-the-key-fragment-each-trustee).

For an automatic key ceremony, the trustees do nothing and the report runs immediately. For the
statuses of the ceremony, see [Tally ceremony](16-admin_portal_tutorials_tally-ceremony.md).

**Expected result:** the status of the report changes to `SUCCESS`. The platform then records
that each election in the report has its initialization report. **Start Online Voting** becomes
available on the **Publish** tab of these elections.

## See and download the report

1. On the **Tally** tab, click the view icon of the report.
2. Click **Results**.
3. Open **Results & Participation**.
4. Select the election, the contest and the area.

The actions menu of each item gives the formats that exist: PDF, HTML, JSON and TAR_GZ. For HTML,
the menu also makes a PDF. For an initialization report, the menu does not give the export of all
the areas of an election in one file. For the formats, see
[Election results](17-admin_portal_tutorials_election-results.md#download-the-result-documents).

Keep the downloaded report with the records of the election.

## If there is a problem

| Problem | Action |
| --- | --- |
| "The Tally Ceremony cannot start until the Key Ceremony has been successfully completed." | Complete the key ceremony. See [Run the Key Ceremony](../03-procedures/04-keys.md). |
| "The Tally Ceremony cannot start until you create one publication in the Publish tab." | Publish the ballot. See [Publish and Manage the Voting Period](../03-procedures/05-publish.md). |
| "You cannot continue the ceremony because no elections are selected or the elections are not published." | Select one or more elections. Make sure that each election is published and has no other initialization report. |
| "Election ...: publish the election before creating its tally." | Publish the election, then start again. |
| "Election ...: initialization reports are not allowed." | The status of the election does not permit the report. Wait for the **Allow Initialization Report** scheduled event, or contact Sequent support. |
| **Start Online Voting** is not available after the report. | Make sure that the report status is `SUCCESS` and that the report includes this election. |
| The election already has a report that you do not want. | Cancel it on the **Tally** tab if its status is `STARTED` or `CONNECTED`. Then make a new report. |
