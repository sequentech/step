---
title: Get the Results
sidebar_position: 7
description: "This procedure shows the results of a tally and downloads the result documents. The election administrator does this procedure."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Get the Results

This procedure shows the results of a tally and downloads the result documents. The election
administrator does this procedure.

## Before you start

- The tally status is `SUCCESS`. See [Run the Tally Ceremony](06-tally.md).
- Your account has the permission to export the results.

## 1. Look at the results

1. Open the election event.
2. Click the **Tally** tab.
3. Click the view icon of the tally.
4. Click **Results**.
5. Open **Results & Participation**.
6. Select the election, then the contest. Under **Areas**, select **Global** or one area.

**Expected result:** the page shows:

- The **Participation Summary**: the eligible voters, the votes counted, and the valid, invalid
  and blank votes, with their percentages.
- The **Candidate Results**: the votes of each candidate and the winning positions.

7. Compare **Total Votes Counted** with **Actual Voters** on the **Dashboard** tab of the
   election.

## 2. Download the result documents

You can download the results of the election event, an election, a contest or an area.

1. On the results page, open the actions menu of the item. The menu items have the form
   "Export in PDF format - 'Election 1' results".
2. Click the format to download. The menu shows the formats that exist for the item.

| Format | Content |
| --- | --- |
| **PDF** | The results document, ready to print and sign. |
| **HTML** | The results document, for a browser. You can also make a PDF from it. |
| **JSON** | The results in a data format, for other systems. |
| **TAR_GZ** | An archive with all the result files. |
| **RECEIPTS_PDF** | The vote receipts. |

3. Keep the files in a safe location.

**Expected result:** the browser saves the file.

## 3. Make the reports

The **Reports** tab makes other documents, for example the electoral results report, a
statistical report or the activity log.

1. Open the election event.
2. Click the **Reports** tab.
3. Click **Add**. If the list is empty, click **Create Report**.
4. In **Type**, select the report type, for example **Electoral Results**.
5. Select the **Election** and the **Template**, if necessary.
6. In **Encryption Policy**, select **Unencrypted** or **Configured Password**.
7. If you select **Configured Password**, type the password in **Password:** and in
   **Repeat Password:**, then click **Save Password**.
8. Click **Save**.
9. In the row of the report, open the actions menu and click **Generate**.

**Expected result:** the task "Generate Report" starts. When it is complete, click
**Download File** in the task panel, or on the **Tasks** tab.

:::caution CAUTION
Keep the report password in a safe location. You cannot change the encryption policy of a
report after you set a password.
:::

## 4. Publish the results

Version 9.0 does not publish the results to a public website. Give the result documents to the
persons who announce the results.

## 5. Keep the records

1. Export the election event for your archive. On the **Data** tab, click **Export**, select
   **Encrypt with Password** and the content to include, then click **Export**.
2. Copy the password that shows in the **Password** window and keep it in a safe location.
3. When the task "Export Election Event" is complete, click **Download File**.
4. Keep the export file, the result documents and the records of the ceremonies together.
5. Keep the key fragments until the end of the period for claims. Then destroy them as your
   rules require.

## If there is a problem

| Problem | Action |
| --- | --- |
| The **Results** button is not available. | The tally is not complete. Wait until the status is `SUCCESS`. |
| A format is not in the menu. | The platform did not make that document for the item. Contact Sequent support. |
| The number of votes is not what you expect. | Do not publish the results. Check the publication, the areas and the **Logs** tab, then contact Sequent support. |
| The actions menu of the results is not there. | Your account does not have the **Export Ceremony** permission. Ask your tenant administrator. |
| "Password and confirm password do not match. Please ensure both fields contain the same password." | Type the same report password in both fields. |

## More information

- [Election event: Tally](../Reference/User-Manual/Election-Management/Election-Event/Tally/election_management_election-event_tally.md)
- [Election event: Reports](../Reference/User-Manual/Election-Management/Election-Event/Reports/election_management_election-event_reports.md)
- [Reports and Templates](../Tutorials/Reports-and-Templates/reports_and_templates.md)
- [Value to Template Association](../Tutorials/Reports-and-Templates/value_to_template_association.md)
- [Election event: Data](../Reference/User-Manual/Election-Management/Election-Event/Data/election_management_election-event_data.md)
- [Election event: Tasks](../Reference/User-Manual/Election-Management/Election-Event/Tasks/election_management_election-event_tasks.md)
