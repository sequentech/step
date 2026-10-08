---
title: "06-RESULTS: Get the results"
sidebar_label: "06-RESULTS: Get the results"
sidebar_position: 6
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# 06-RESULTS: Get the results

This procedure shows the results of a tally and downloads the result documents. The election
administrator does this procedure.

## Before you start

- The tally status is `SUCCESS`. See [05-TALLY](05-tally.md).
- Your account has the permission to export the results.

## 1. Look at the results

1. Open the election event.
2. Click the **Tally** tab.
3. Click the view icon of the tally.
4. Click **Results**.
5. Open **Results & Participation**.
6. Select the election, the contest and the area to see.

**Expected result:** the page shows:

- The **Participation Summary**: the eligible voters, the votes counted, and the valid, invalid
  and blank votes, with their percentages.
- The **Candidate Results**: the votes of each candidate and the winning positions.

7. Compare the number of votes counted with the number of votes cast in the **Dashboard** tab.

## 2. Download the result documents

You can download the results of the election event, an election, a contest or an area.

1. On the results page, open the actions menu of the item.
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
3. Click **Create Report**.
4. Select the **Report Type**, for example **Electoral Results**.
5. Select the **Election** and the **Template**.
6. In **Encryption Policy**, select **Configured Password** to protect the file with a
   password, or **Unencrypted**.
7. Click the save button.
8. In the row of the report, click **Generate**.

**Expected result:** the report is available to download when its task is complete.

## 4. Publish the results

Version 9.0 does not publish the results to a public website. Give the result documents to the
persons who announce the results.

## 5. Keep the records

1. Export the election event for your archive. On the **Data** tab, click the export button,
   then select the content to include. Select **Encrypt with Password**.
2. Keep the export file, the result documents and the records of the ceremonies together.
3. Keep the key fragments until the end of the period for claims. Then destroy them as your
   rules require.

## If there is a problem

| Problem | Action |
| --- | --- |
| The **Results** button is not available. | The tally is not complete. Wait until the status is `SUCCESS`. |
| A format is not in the menu. | The platform did not make that document for the item. Contact Sequent support. |
| The number of votes is not what you expect. | Do not publish the results. Check the publication, the areas and the **Logs** tab, then contact Sequent support. |
