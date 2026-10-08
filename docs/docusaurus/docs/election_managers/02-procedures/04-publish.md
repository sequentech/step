---
title: "04-PUBLISH: Publish the ballot and manage the voting period"
sidebar_label: "04-PUBLISH: Publish and voting period"
sidebar_position: 4
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# 04-PUBLISH: Publish the ballot and manage the voting period

This procedure makes the ballot available in the voting portal, then opens, pauses and closes
the voting period. The election administrator does this procedure.

## Before you start

- The content of the election event is complete and correct. See [02-EVENT](02-event.md).
- The key ceremony status is `SUCCESS`. See [03-KEYS](03-keys.md).
- Your account has the permissions to publish and to change the voting status.
- You know your password. Publication and the voting status are sensitive actions. See
  [00-START](../01-before-you-start.md#sensitive-actions).

:::danger WARNING
Do not publish before the key ceremony status is `SUCCESS`. Version 9.0 does not stop you. A
ballot that you publish without the public key cannot be encrypted correctly.
:::

## 1. Make a publication

1. Open the election event.
2. Click the **Publish** tab.
3. Click **Generate Publication**. If the election event already has a publication, click
   **Publish Changes**.
4. Read the message "You are about to generate a publication."
5. Click **Confirm**. If the admin portal asks for your password, type it.

**Expected result:** the message "Ballot generated" shows. The **Changes to be Published** page
compares the **Current** publication with the **Changes to Publish**.

6. Read the changes. Make sure that they are the changes that you expect.

## 2. Preview the ballot

1. On the **Changes to be Published** page, click **Preview**.
2. In **Select Area for Preview**, select an area.
3. Open the preview. To send the preview to another person, click **Copy link**.
4. Check the ballot of the area: the elections, the contests, the candidates, the texts in each
   language and the number of selections.
5. Do steps 2 to 4 for each area.

If you find an error, correct it in the election event. Then start again at step 1 of
[Make a publication](#1-make-a-publication).

## 3. Publish the ballot

1. On the **Changes to be Published** page, click **Publish Changes**.
2. Confirm the action.

**Expected result:** the message "Ballot published" shows. The **Publish History** list shows
the new publication.

:::note
To change the ballot after you publish it, change the election event and do this procedure
again from step 1. Voters see the last publication.
:::

## 4. Make the initialization report (if necessary)

Do this step only if **Initialize Report Policy** of an election is **Required**. The
initialization report shows that the ballot box is empty before the voting period opens.
**Start Voting** stays unavailable until the report exists.

1. Open the election event.
2. Click the **Tally** tab.
3. Click **Generate Initialization Report**.
4. Select the elections in **Elections for Initialization Report**.
5. Click **Start Initialization Report**.
6. Continue as in a tally ceremony. See [05-TALLY](05-tally.md).

**Expected result:** the **Tally** list shows a row with the **Tally Type**
**Initialization Results**.

## 5. Open the voting period

Do this step at the time of the start of the voting period. To open the voting period
automatically, use [scheduled events](#7-schedule-the-voting-period-alternative).

1. Open the election event.
2. Click the **Publish** tab.
3. Click **Start Voting**.
4. Read the message "You are about to start voting period."
5. Confirm the action. If the admin portal asks for your password, type it.

**Expected result:** the message "Election status changed" shows. Voters can vote in the
voting portal on the voting channels of the election event.

6. Sign in to the voting portal with a test voter account, if you have one, and make sure that
   the ballot shows.

## 6. Pause or close the voting period

To stop the voting for a short time:

1. On the **Publish** tab, click **Pause Voting**.
2. Confirm the action.
3. To continue, click **Start Voting** and confirm the action.

To close the voting period at its end:

:::danger WARNING
In version 9.0 you cannot open the voting period again from the admin portal after you close
it. Close the voting period only at its official end.
:::

1. On the **Publish** tab, click **Stop Voting**. This closes the **Online** channel.
2. Confirm the action.
3. If the election event uses kiosks, click **Stop Kiosk Voting** and confirm the action.

**Expected result:** the message "Election status changed" shows. Voters cannot vote.

## 7. Schedule the voting period (alternative)

Use scheduled events to open and close the voting period at a set time.

1. Open the election event.
2. Click the **Scheduled Events** tab.
3. Click **Create Scheduled Event**.
4. In **Type**, select **Start Voting Period**.
5. In **Election**, select an election. Leave it empty for all the elections.
6. Type the date and the time. The admin portal uses the time zone of your computer. The label
   of the field shows the time zone.
7. Click the save button.
8. Do steps 3 to 7 again with the type **End Voting Period**.

**Expected result:** the **Scheduled Events** tab shows the two events.

:::caution CAUTION
Check the time zone in the label of the date field. If your computer is not in the time zone of
the election, convert the times.
:::

## If there is a problem

| Problem | Action |
| --- | --- |
| "Election event is locked down" | Set **Lockdown Status** to **Not Locked Down** on the **Data** tab. See [02-EVENT](02-event.md). |
| "Ballot publication not generated yet, can't publish." | Make a publication first. See [step 1](#1-make-a-publication). |
| "Insufficient privileges" | Confirm your identity again with your password, then repeat the action. |
| **Start Voting** is not available. | Make the initialization report, if the election needs one. If the voting period is closed, contact Sequent support. |
| **Stop Voting** is not available. | A scheduled event does not permit the end of the voting period yet. Look at the **Scheduled Events** tab. |
| Voters do not see a contest. | Make sure that the contest is in the area of the voter. Then make and publish a new publication. |

**Next:** [05-TALLY: Run the tally ceremony](05-tally.md).
