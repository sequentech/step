---
id: admin_portal_tutorials_tally-ceremony
title: Tally Ceremony
description: "The main path is in Run the Tally Ceremony. This page gives the details: the statuses, the policies that block or permit a tally, the automatic tally, the resolution of ties, the recount and the…"
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The main path is in [Run the Tally Ceremony](../03-procedures/05-tally.md). This page gives the details: the
statuses, the policies that block or permit a tally, the automatic tally, the resolution of ties,
the recount and the cancellation.

## Types of tally

A tally on the **Tally** tab has one of two types, in the column **Tally Type**:

| **Tally Type** | Made with | Content |
| --- | --- | --- |
| **Electoral Results** | **Start Tally Ceremony** | The count of the votes after the voting period. |
| **Initialization Results** | **Generate Initialization Report** | The count of the empty ballot box before the voting period. See [Initialization report](11-admin_portal_tutorials_initialization-report.md). |

The **Tally** list also has the columns **Permission Labels**, **Trustees**,
**Number Elections** and **Status**.

## The steps of the wizard

The tally page shows its steps at the top: **Start**, **Ceremony**, **Tally** and **Results**. For
an automatic key ceremony, the **Ceremony** step is not shown.

| Step | What you do |
| --- | --- |
| **Start** | Select the elections and the key ceremony, then create the tally. |
| **Ceremony** | Watch the trustees upload their key fragment, then start the tally. |
| **Tally** | Watch the progress of each election. Resolve ties, if necessary. |
| **Results** | Look at the results, download the documents and publish them. See [Election results](17-admin_portal_tutorials_election-results.md). |

When you open a tally from the list, the page goes to the step that matches its status.

## Tally status

| Status | Meaning | Next status |
| --- | --- | --- |
| `STARTED` | The tally is created. The trustees can upload their key fragment. | `CONNECTED` when enough trustees uploaded, or `CANCELLED`. |
| `CONNECTED` | The number of trustees that uploaded their fragment is equal to or more than the threshold. | `IN_PROGRESS` when you click **Start Tally**, or `CANCELLED`. |
| `IN_PROGRESS` | The platform mixes, decrypts and counts the votes. | `SUCCESS`, `AWAITING_INPUT` or `CANCELLED`. |
| `AWAITING_INPUT` | The count stopped at a tie that needs a decision. | `IN_PROGRESS` after the resolution, or `CANCELLED`. |
| `SUCCESS` | The tally is complete. | `IN_PROGRESS` for a recount. |
| `CANCELLED` | The tally is cancelled. | None. |

The **Elections Tally Progress** table shows a status for each election:

| Election status | Meaning |
| --- | --- |
| `WAITING` | The election waits for its turn. |
| `MIXING` | The platform shuffles the encrypted votes, so that no vote can be linked to a voter. |
| `DECRYPTING` | The platform decrypts the votes with the key fragments. |
| `SUCCESS` | The count of the election is complete. |
| `ERROR` | The count of the election failed. Contact Sequent support. |

## The threshold and the trustees

The threshold is the minimum number of trustees that the key ceremony set. The **Trustees** section
of the **Ceremony** step shows the number of trustees that imported their key and the number that
is necessary. The tally becomes `CONNECTED` when the first number reaches the second.

You do not need all the trustees. You need a number of trustees equal to or more than the
threshold.

### What the trustees see

- On the **Tally** tab, a trustee sees the message "You have been invited to participate in a
  Tally ceremony." while the tally waits for the key fragments.
- The key icon **Add Tally Key** shows only while the trustee has not uploaded a fragment and the
  tally is `STARTED` or `CONNECTED`. In all other cases, the trustee sees **View Tally Ceremony**.
- The trustee does not start the tally. The election administrator clicks **Start Tally**.

## Policies that block or permit the tally

Each election has the field **Allow Tally** on its **Data** tab.

| **Allow Tally** | Effect |
| --- | --- |
| **Allowed** (default) | You can create and start the tally at any time, also while voters vote. |
| **Disallowed** | You cannot tally the election. |
| **Requires Voting Period End** | You can tally the election only when online voting is `CLOSED` and each other allowed channel is `CLOSED` or `NOT_STARTED`. |

The scheduled event **Allow Tally** sets the field to **Allowed** at a set time. See
[Scheduled events](13-admin_portal_tutorials_scheduled-events.md).

:::danger WARNING
Use **Requires Voting Period End** for each election. With **Allowed**, a person can start the
count while voters still vote.
:::

The platform checks these rules two times: when you create the tally and when you click **Start
Tally**. Each selected election must also be published.

Other rules:

- All the selected elections must use the same key ceremony. Make one tally for each key ceremony.
- You see and tally only the elections with a permission label that your account has.
- Delegated voting and voter-weighted voting cannot both be enabled in the same election event.
  Voter-weighted voting has other limits on the counting method and on tally sheets. The platform
  shows the cause when it refuses the tally.

## Automatic tally

If the key ceremony is automatic, the tally does not need trustees:

- The button on the **Start** step is **Start Tally**.
- The message is "Select Start Tally to run tally process and display results, or Close to
  cancel."
- The tally starts with the status `IN_PROGRESS`. The **Ceremony** step is not shown.

An automatic key ceremony is possible only when **Keys/Tally Ceremonies Policy** of the election
event permits automatic ceremonies.

## Resolve a tie

A contest can stop at a tie, for example an **Instant Runoff** contest with the
**Tie-Breaking Policy** **External Procedure**. The status of the tally then changes to
`AWAITING_INPUT`. The tally page shows the **Pending resolutions** panel and the message "Tally
paused due to unresolved tie (Round ...)".

Your account needs the permission **Submit tally resolution**.

1. In the **Pending resolutions** list, click a tie. Use **Filter** to find it by **Election**,
   **Contest**, **Area** or **Status**.
2. Read the tied candidates and their votes.
3. In **Select candidate to advance**, select the candidate that your decision gives.
4. Click **Save**. The tie shows the status **Pending calculation**.
5. Do steps 1 to 4 for each tie.
6. Click **Apply Resolutions and Recalculate**.

**Expected result:** the message "Resolutions submitted. Tally is resuming..." shows. The status
changes to `IN_PROGRESS`. The resolved ties show "Tally resumed after resolution applied", with the
date and the user.

To change a decision before you apply it, click **Undo Resolution**. The platform records each
pause, each resolution and each restart in the electoral log.

## Recount a tally

A recount counts a complete tally again and makes a new set of results. Use it, for example, after
the approval of new tally sheets.

- The action **Recount tally** shows only for a tally with the status `SUCCESS`.
- Your account needs the permission **Execute Tally Recount**.
- The status changes to `IN_PROGRESS`, the progress of each election starts again at `WAITING`,
  and then the status changes to `SUCCESS`.

## Cancel a tally

**Cancel Tally Ceremony** shows in the list only when the status is `STARTED` or `CONNECTED`. The
dialog says "You are about to cancel the tally ceremony. This action is not undoable." Click
**Cancel Tally** to confirm.

:::caution CAUTION
You cannot undo a cancellation. For a new tally, the trustees must upload their key fragment again.
:::

## Permissions

| Permission | Lets you |
| --- | --- |
| **View Election Event Tally** and **Read Tally** | See the **Tally** tab. |
| **Create Ceremony** | See **Start Tally Ceremony** and **Generate Initialization Report**. |
| **Admin Ceremony** | Create, start, view and cancel tallies. |
| **Trustee Ceremony** | Upload a key fragment as a trustee. |
| **Submit tally resolution** | Resolve ties. |
| **Execute Tally Recount** | Recount a complete tally. |
| **Export Ceremony** | Download the result documents. |

## If there is a problem

| Problem | Action |
| --- | --- |
| **Start Tally Ceremony** is not available. | Complete the key ceremony and make a publication. Wait until the page shows no warning. |
| "Select at least one election." | Select one or more elections. |
| "Publish each selected election before creating its tally." | Publish the ballot. See [Publish and Manage the Voting Period](../03-procedures/04-publish.md). |
| "Tallying is disabled for a selected election." | **Allow Tally** of the election is **Disallowed**. Change it, or wait for the **Allow Tally** scheduled event. |
| "End voting in each selected election and stop its active voting channels before creating the tally." | Close online voting and each other open or paused channel of the election. |
| "Elections have different keys ceremonies" | Make one tally for each key ceremony. |
| "Some elections don't have the required permission label or are not published" | Publish the elections. Ask for the permission label of the elections. |
| "You cannot continue the ceremony because the tally session is not connected or the start of the ceremony is not allowed." | Wait for more trustees. If the status is `CONNECTED`, check **Allow Tally** and the voting channels. |
| "Insufficient number of connected trustees ..." | Wait until enough trustees upload their fragment. |
| "Trustee not part of the keys ceremony or has invalid state" | The trustee is not in the key ceremony of these elections, or has already uploaded the fragment. |
| "Invalid Encrypted Private Key Backup, please try again" | The trustee uploaded a wrong file. Use the key fragment file of this election event. |
| "Your key was already restored." | The trustee has already uploaded the fragment. No action is necessary. |
| "Failed to submit resolutions. Please try again." | Try again. Make sure that your account has **Submit tally resolution**. |
| "Could not start recount" | Make sure that the status is `SUCCESS`, then try again. |
| An election shows `ERROR`. | Do not start a new tally. Contact Sequent support and give the name of the election event. |
