---
id: election_management_contest_tally_sheets
title: Tally Sheets
description: "The Tally Sheets tab of a contest holds the results of votes that were not cast online, for example paper or postal votes."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Tally Sheets** tab of a contest holds the results of votes that were not cast online, for
example paper or postal votes. Each tally sheet gives the results of one contest in one area
for one channel. The tally adds the published tally sheets to the online results.

## Open the tab

1. In the menu on the left, open the election event and the election.
2. Click the contest.
3. Click the **Tally Sheets** tab.

The actions need these permissions:

| Action | Permission |
| --- | --- |
| See the list and view a sheet | **View Tally Sheet** |
| Create and edit a sheet | **Create Tally Sheet** |
| Publish and unpublish a sheet | **Publish Tally Sheet** |
| Delete a sheet | **Delete Tally Sheet** |

## List

When the list is empty, the tab shows "No Tally Sheet Yet." and **Generate Tally Sheet**. When
there are sheets, the toolbar has **Add**.

| Column | Content |
| --- | --- |
| **Id** | The identifier of the sheet. |
| **Channel** | `PAPER` or `POSTAL`. |
| **Contest** | The contest. |
| **Area** | The area. |
| **Published** | If the sheet is published. |
| **Actions** | Edit, view, **Publish** or **Unpublish**, and delete icons. |

## Create a tally sheet

The wizard has the steps **Edit** and **Confirm**.

1. Click **Add**. If the list is empty, click **Generate Tally Sheet**.
2. In **Search Area**, select the area.
3. In **Channel**, select `PAPER` or `POSTAL`.
4. Type the numbers: **Total Votes**, **Total Valid Votes**, **Total Invalid Votes**,
   **Implicitly Invalid Votes**, **Explicitly Invalid Votes**, **Blank Votes** and **Census**.
5. Under **Candidates**, type the votes of each candidate.
6. Click **Next**.
7. Check the numbers on the confirmation page.
8. Click **Save**.

**Expected result:** the message "Tally Sheet saved" shows.

9. In the row of the sheet, click the publish icon. Read "Are you sure tu publish this Tally
   Sheet?" and confirm.

**Expected result:** the message "Tally sheet published" shows.

:::note
Only published tally sheets go into the tally. Publish each sheet before you start the tally.
:::

## If there is a problem

| Problem | Action |
| --- | --- |
| "All fields are required" | Type a value in each field, then click **Next**. |
| "Error saving Tally Sheet" | Check the numbers, then save again. |
| "Error publishing tally sheet" | Try again. |
| A field label shows a text code such as `tallysheet.label.contest_id`. | The label is not translated in version 9.0. The field shows the contest. |

## Related pages

- [Data](../Data/election_management_contest_data.md)
- [Election event: Tally](../../Election-Event/Tally/election_management_election-event_tally.md)
- [Run the Tally Ceremony](../../../../../procedures/06-tally.md)
