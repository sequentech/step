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

This procedure shows the results of a tally, downloads the result documents and publishes the
results on the results website. The election administrator does this procedure.

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

- The **Participation Summary**: the eligible voters, the voters, and the valid, invalid and
  blank votes, with their percentages.
- The participation by channel.
- The **Candidate Results**: the votes of each candidate and the winning positions. For an
  **Instant Runoff** contest, the page also shows each round, the eliminated candidates and the
  winner.

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

For an election, the menu can also export the results of all its areas in one HTML or JSON
file. For the election event, you can also export the results in **XLSX** format, for a
spreadsheet.

3. Keep the files in a safe location.

**Expected result:** the browser saves the file.

## 3. Make the reports

The **Reports** tab makes other documents, for example the electoral results report, a
statistical report or the activity log.

1. Open the election event.
2. Click the **Reports** tab.
3. Click **Create Report**.
4. Select the **Report Type**, for example **Electoral Results** or **Participation Report**.
5. Select the **Election** and the **Template**.
6. In **Encryption Policy**, select **Configured Password** to protect the file with a
   password, or **Unencrypted**.
7. Click the save button.
8. In the row of the report, click **Generate**.

**Expected result:** the report is available to download when its task is complete.

## 4. Publish the results on the results website

Do this step only if your election event uses the results website.

### Before you publish

- **Results Website** is **Enabled** on the **Data** tab of the election event. See
  [02-EVENT](02-event.md#2-configure-the-election-event).
- The tally status is `SUCCESS`.
- Your account has the permission to publish the results.
- The persons responsible for the election have approved the results.

:::caution CAUTION
The public can see the results immediately after the publication. Check the results before you
publish them.
:::

1. On the results page of the tally, open **Publish to results website**.
2. In **Route**, select **Event results** or **Election results**. For **Election results**,
   select the **Election**.
3. In **Access**, select **Public access** or **Authenticated access**.
4. In **Visibility**, select the scope of the results.
5. In **Contests**, select the contests to publish.
6. Click **Publish selected contests**.
7. Read the message "Start publish to results website?" and confirm.

**Expected result:** the message "Results publication started" shows. The **Publication
history** shows a new version.

8. In **Publication history**, click **Open** to see the published page.

To remove a publication from the results website, click **Revoke** in its row of the
**Publication history**.

For more information, see [Results website](../03-more-procedures/30-results-website.md).

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
| "Results website publishing is disabled for this election event." | Set **Results Website** to **Enabled** on the **Data** tab of the election event. |
| "Results can be published after this tally has completed." | Wait until the tally status is `SUCCESS`. |
| A format is not in the menu. | The platform did not make that document for the item. Contact Sequent support. |
| The number of votes is not what you expect. | Do not publish the results. Check the publication, the areas and the **Logs** tab, then contact Sequent support. |
