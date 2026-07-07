---
id: admin_portal_tutorials_ballot-boxes
title: Ballot Boxes (Tally Sheets)
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This tutorial explains how to digitalize results from non-electronic voting channels (such as **Paper** or **Postal** ballots) using **Ballot Boxes**, and how to review and approve them so they are included in the final tally alongside electronic votes.

:::info
A **Ballot Box** (internally called a **Tally Sheet**) is the digitalized vote count for one unique combination of **Contest**, **Area**, and **Channel**. Every time you submit new figures for that combination, a new **version** is created — nothing is overwritten, so you always keep a full audit trail of who entered and who reviewed each version.
:::

## Step 1: Open the Tally Sheets Tab

Ballot boxes are managed at the **Election** level (not per contest).

1. Navigate to the **Election Event** and select the specific **Election**.
2. Select the **Tally Sheets** tab.

The page header reads **Ballot boxes — Digitalized ballot boxes by channel**.

:::info
This tab is only visible to users with the `View Tally Sheet` permission. Creating ballot boxes requires `Create Tally Sheet`, and approving or disapproving versions requires `Review Tally Sheet`.
:::

## Step 2: Understand the Ballot Boxes List

The main list shows one row per unique **Contest + Area + Channel** combination, with:

* **Channel**: e.g. `PAPER` or `POSTAL`.
* **Contest** and **Area** the ballot box belongs to.
* **Latest version**: the version number of the most recently submitted figures.
* **Approved version**: the version number that has been approved and will be used in the final tally (`-` if none has been approved yet).

Each row has two actions available from the actions menu:

* **Add**: submit a new version of figures for that ballot box.
* **Versions**: open the full version history for that ballot box.

## Step 3: Digitalize a New Ballot Box

1. Select the **+ Add** button above the list (or, if no ballot boxes exist yet, **Generate Tally Sheet** on the empty state).
2. Choose the **Contest**, **Area**, and **Channel** the figures belong to.
3. Enter the vote counts:
   * **Total Valid Votes**
   * **Implicitly Invalid Votes** and **Explicitly Invalid Votes** (the **Total Invalid Votes** field is calculated automatically as their sum)
   * **Blank Votes**
   * **Census** (must be entered manually and must be greater than or equal to the total votes)
   * The number of votes for each **Candidate**
4. Select **Confirm**. The system validates that the sum of candidate votes plus blank votes equals the total valid votes, and that the census is large enough; if not, an error is shown and you cannot proceed.
5. Review the read-only summary and select **Save**.

This creates **version 1** of the ballot box, with status **Pending**.

:::tip
You cannot create a second, independent ballot box for the same Contest + Area + Channel combination. To correct or update figures for an existing ballot box, use the **Add** action from the list instead (see next step).
:::

## Step 4: Submit a New Version (Correction)

If figures need to be corrected (for example, after a recount), do not start a new ballot box — add a new version to the existing one:

1. From the Ballot Boxes list, select **Add** on the row of the ballot box you want to update.
2. The form opens pre-filled with the values of the latest version. Edit the fields as needed.
3. Select **Confirm**, review the summary, and **Save**.

This creates a new version (e.g. version 2) with status **Pending**, without affecting the previous versions yet.

## Step 5: Review Version History

1. From the Ballot Boxes list, select **Versions** on the row of the ballot box you want to inspect.
2. The versions table shows, for every version submitted for that Contest + Area + Channel:
   * **Version** number
   * **Created by** and **Created at**
   * **Reviewed by** and **Reviewed at** (`-` until reviewed)
   * **Status**: `PENDING`, `APPROVED`, or `DISAPPROVED`
3. Use **Show** to view the read-only figures of any version.
4. Select **Back** to return to the Ballot Boxes list.

## Step 6: Approve or Disapprove a Version

Pending versions can be approved or disapproved from the versions table (requires the `Review Tally Sheet` permission):

1. Select **Approve** or **Disapprove** on a pending version.
2. Confirm the action in the dialog.

:::info
Approving a version sets its status to **Approved** and records the reviewer and review time. It also automatically discards the other versions of that same ballot box, so only one version per ballot box remains as the authoritative, approved record.
:::

Disapproving a version marks it as **Disapproved**; you can still submit a new corrected version afterward using **Add**.

:::tip
Only **Approved** ballot boxes are included when the Tally Ceremony is executed — their figures are merged with the electronically cast and decrypted votes to produce the final results. Make sure every relevant ballot box has an approved version before running the tally.
:::
