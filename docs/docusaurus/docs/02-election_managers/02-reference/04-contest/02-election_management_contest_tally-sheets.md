---
id: election_management_contest_tally_sheets
title: Tally Sheets
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Tally Sheets

:::note
The contest has no **Tally Sheets** tab in this version of the admin portal. The contest has
only the **Data** tab. Tally sheets are on the **Tally Sheets** tab of the **election**. This
page describes that tab.
:::

A tally sheet holds the results of one ballot box that the platform did not count, for example
paper ballots or postal ballots. An election administrator types the results of the ballot box
into the admin portal. A second person with review permission approves or disapproves the tally
sheet. The tally adds the results of the approved tally sheets to the results of the online
votes.

A ballot box is one combination of a contest, an area and a channel. Each ballot box can have
more than one tally sheet. Each tally sheet of a ballot box is a **version**.

To import tally sheets from a file, use the **Tally sheet imports** tab of the election event.
See [Tally Sheet Imports](../02-election-event/08-03-election_management_election-event_tally-sheet-imports.md).

## Open the tab

1. In the menu on the left, open the election event.
2. Click the election.
3. Click the **Tally Sheets** tab.

The tab shows the title **Ballot boxes** and the text "Digitalized ballot boxes by channel".

## Permissions

| Permission (in plain words) | What you can do |
| --- | --- |
| See tally sheets | See the **Tally Sheets** tab, the ballot boxes and their versions. |
| Create tally sheets | Make a tally sheet, or a new version of a tally sheet. |
| Review tally sheets | Approve or disapprove a tally sheet version. |
| See tally sheet imports | See the import that made a version, and download its source file. |

:::danger WARNING
Give the permission to create tally sheets and the permission to review tally sheets to
different persons. Then a second person checks each tally sheet before the tally counts it.
:::

## Ballot box list

The list shows one row for each ballot box of the election.

| Column | Meaning |
| --- | --- |
| **Id** | The identifier of a tally sheet of the ballot box. Hidden by default. Use **Columns** to show it. |
| **Channel** | The channel of the ballot box: `PAPER` or `POSTAL`. |
| **Contest** | The contest of the ballot box. |
| **Area** | The area of the ballot box. |
| **Latest version** | The number of the newest version. |
| **Approved version** | The number of the approved version. A dash (-) shows if no version is approved. |
| **Labels** | Labels of the tally sheet, if any. |
| **Annotations** | Annotations of the tally sheet, if any. |
| **Actions** | **Add** makes a new version from the latest version. **Versions** opens the list of versions. |

| Button | Use |
| --- | --- |
| **Add** | Make a tally sheet for a new ballot box. |
| **Generate Tally Sheet** | Shows when the election has no tally sheet ("No Tally Sheet Yet."). Makes the first tally sheet. |
| **Columns** | Show or hide columns. |
| **Add filter** | Filter by **Area**, **Contest**, **ID**, **Channel**, **Latest version**, **Labels** or **Annotations**. |

## Tally sheet form

The form has the title **Tally Sheet**. The steps at the top are **Edit** and **Confirm**. For a new
ballot box, the button to go to the **Confirm** step is **Next**. For a new version, it is
**Confirm**.

| Field | Meaning |
| --- | --- |
| **Search Contest** | The contest of the ballot box. Required. |
| **Search Area** | The area of the ballot box. The list shows the areas of the selected contest. Required. |
| **Channel** | `PAPER` or `POSTAL`. Required. |
| **Total Votes** | Calculated: **Total Valid Votes** plus **Total Invalid Votes**. You cannot type it. |
| **Total Valid Votes** | The number of valid votes. Required. |
| **Total Invalid Votes** | Calculated: **Implicitly Invalid Votes** plus **Explicitly Invalid Votes**. You cannot type it. |
| **Implicitly Invalid Votes** | The number of implicitly invalid votes in the ballot box. Required. |
| **Explicitly Invalid Votes** | The number of explicitly invalid votes in the ballot box. Required. |
| **Blank Votes** | The number of blank votes in this contest. Required. |
| **Blank Ballots** | The number of fully blank ballots in the ballot box. Optional. It must be the same on each contest of the ballot box. |
| **Census** | The number of voters of the ballot box. Required. **Total Votes** must not be more than **Census**. |
| **Candidates** | One **Total Votes** field for each candidate. Required. |

The admin portal checks the numbers while you type. If a number is not correct, a red message
shows under the field, and you cannot go to the next step.

## Make a tally sheet

1. On the **Tally Sheets** tab, click **Add**. If the election has no tally sheet, click
   **Generate Tally Sheet**.
2. In **Search Contest**, select the contest.
3. In **Search Area**, select the area.
4. In **Channel**, select the channel.
5. Type the numbers of the ballot box in each field.
6. Type the votes of each candidate.
7. Make sure that no red message shows.
8. Click **Next**.
9. Compare the numbers on the screen with the paper record of the ballot box.
10. If a number is not correct, click **Back** and correct it.
11. Click **Save**.

**Expected result:** the message "Tally Sheet saved" shows. The ballot box shows in the list.
The new version has the status `PENDING`.

## Correct a tally sheet

You cannot change a saved version. To correct a tally sheet, make a new version.

1. In the row of the ballot box, click **Add**.
2. Correct the numbers. The contest, the area and the channel stay the same.
3. Click **Confirm**.
4. Click **Save**.

**Expected result:** the message "Tally Sheet saved" shows. The **Latest version** number goes
up by one. The new version has the status `PENDING`.

## Versions

Click **Versions** in the row of a ballot box. The page "Versions for ballot box" opens. It shows
the channel, the area and the contest, and one row for each version, the newest first.

| Column | Meaning |
| --- | --- |
| **Version** | The number of the version. |
| **Created by** | The user who made the version. |
| **Created at** | The date and time when the user made the version. |
| **Reviewed by** | The user who approved or disapproved the version. |
| **Reviewed at** | The date and time of the review. A dash (-) shows if nobody reviewed it. |
| **Status** | The review status. The table below gives the values. |
| **Labels** | Labels of the version, if any. |
| **Annotations** | Annotations of the version, if any. |
| **Source import** | For a version made by an import: the status of the import, a link to open the import and a link to download the source file. A dash (-) shows for a version that you typed. |
| **Actions** | **Show**, **Approve** and **Disapprove**. |

| **Status** | Meaning |
| --- | --- |
| `PENDING` | The version waits for a review. The tally does not count it. |
| `APPROVED` | A reviewer approved the version. The tally counts it. |
| `DISAPPROVED` | A reviewer disapproved the version. The tally does not count it. |

Click **Back** to go back to the ballot box list.

## Review a tally sheet

You need permission to review tally sheets. The **Approve** and **Disapprove** actions show only
for a version with the status `PENDING`.

:::caution CAUTION
Approve only the version that is correct. When you approve a version, the platform removes all
the other versions of the ballot box, also newer versions that wait for a review.
:::

1. On the **Tally Sheets** tab, click **Versions** in the row of the ballot box.
2. In the row of the version, click **Show**.
3. Compare the numbers with the paper record of the ballot box.
4. Click **Back**.
5. Click **Versions** in the row of the ballot box.
6. In the row of the version, click **Approve** or **Disapprove**.
7. Read the message "Are you sure to approve this Tally Sheet?" or "Are you sure to disapprove
   this Tally Sheet?".
8. Click **Approve** or **Disapprove**.

**Expected result:** the message "Tally sheet reviewed" shows. The status of the version changes.

:::note
The tally reads the approved tally sheets when it runs. If you approve a tally sheet after a
tally is complete, count the tally again. See
[Count again](../../03-procedures/05-tally.md#count-again).
:::

## If there is a problem

| Problem | Action |
| --- | --- |
| "All fields are required" | Fill in all the required fields. Then click **Next** or **Confirm** again. |
| **Next** or **Confirm** is not available. | Read the red messages under the fields. Correct the numbers. |
| "Total votes (…) must not be greater than census (…)" | Make sure that **Census** and the vote numbers are correct. |
| "Candidate votes (…) must be between … and … for this contest's voting rules (…)" | The total of the candidate votes does not agree with **Total Valid Votes**, **Blank Votes** and the number of marks for each ballot. Correct the numbers. |
| "Blank Ballots must have the same value on every contest sheet of this ballot box" | Use the same **Blank Ballots** number on each contest of the ballot box. This message is a warning. It does not stop the save. |
| "Blank Ballots value is outside the range implied by this box's per-contest blank vote counts" | Make sure that **Blank Ballots** agrees with the **Blank Votes** of each contest. This message is a warning. |
| "This contest's counting algorithm (…) is not recognised, …" | Check the configuration of the contest. |
| "Error saving Tally Sheet" | Make sure that you have permission to create tally sheets. An acclaimed contest cannot have tally sheets. See [Acclaimed contests](../11-acclaimed-contests.md). |
| "Error reviewing tally sheet" | Make sure that you have permission to review tally sheets. Only a `PENDING` version can be reviewed. |
| The tally fails with a message that starts with "Approved tally sheets cannot be counted when voter-weighted voting is enabled". | The election event uses voter-weighted voting. Tally sheets cannot be used with it. Contact Sequent support. |

## Related pages

- [Tally Sheet Imports](../02-election-event/08-03-election_management_election-event_tally-sheet-imports.md)
- [Contest data](01-election_management_contest_data.md)
- [Run the Tally Ceremony](../../03-procedures/05-tally.md)
