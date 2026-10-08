---
title: Publish and Manage the Voting Period
sidebar_position: 4
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Publish and Manage the Voting Period

This procedure makes the ballot available in the voting portal, then opens, pauses and closes
the voting period. The election administrator does this procedure.

## Before you start

- The content of the election event is complete and correct. See [Create the Election Event](02-event.md).
- The key ceremony status is `SUCCESS`. See [Run the Key Ceremony](03-keys.md).
- Your account has the permissions to publish and to change the voting status.
- You know your password. Publication and the voting status are sensitive actions. See
  [Before You Start](../00-before-you-start.md#sensitive-actions).

:::danger WARNING
Do not publish the ballot for voters before the key ceremony status is `SUCCESS`. The admin portal
does not stop you. Without the election key, the voting portal works in demo mode and does not
record real votes.
:::

:::note
The **Publish** tab of the election event applies to all its elections. Each election also has
a **Publish** tab that applies only to that election.
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
On the **Publish** tab of the election, **Start Online Voting** stays unavailable until the
report exists. See [Initialization report](../01-tutorials/11-admin_portal_tutorials_initialization-report.md).

1. Open the election event.
2. Click the **Tally** tab.
3. Click **Generate Initialization Report**.
4. Select the elections in **Elections for Initialization Report**.
5. Click **Start Initialization Report**.
6. Continue as in a tally ceremony. See [Run the Tally Ceremony](05-tally.md).

**Expected result:** the **Tally** list shows a row with the **Tally Type**
**Initialization Results**.

## 5. Open the voting period

Each voting channel has its own status. Open each channel of the election event. To open the
voting period automatically, use [scheduled events](#7-schedule-the-voting-period-alternative).
For all the statuses and rules, see [Start, pause and stop the voting](../01-tutorials/14-admin_portal_tutorials_start-pause-stop-election_copy.md).

:::caution CAUTION
The **Publish** tab of the election event does not check the initialization report or the end
of the voting period. If an election needs these checks, open and close its channels on the
**Publish** tab of the election.
:::

1. Open the election event.
2. Click the **Publish** tab.
3. Click **Start Voting**. A menu shows one item for each channel.
4. Click the channel to open, for example **Start Online Voting**.
5. Read the message "You are about to start voting period."
6. Confirm the action. If the admin portal asks for your password, type it.
7. Do steps 3 to 6 for each other channel of the election event.

**Expected result:** the message "Election status changed" shows. Voters can vote on the
channels that you opened.

:::note
A menu item is not available if the channel is not allowed in **Voting Channels Allowed**. Open
**Early Voting** before **Online Voting**. You cannot start early voting after online voting has
started.
:::

8. Sign in to the voting portal with a test voter account, if you have one, and make sure that
   the ballot shows.

## 6. Pause or close the voting period

To stop the voting for a short time:

1. On the **Publish** tab, click **Pause Voting**.
2. Click the channel, for example **Pause Online Voting**.
3. Confirm the action.
4. To continue, click **Start Voting**, click the channel and confirm the action.

To close the voting period at its end:

:::danger WARNING
You cannot open a channel again from the admin portal after you close it. Close the voting
period only at its official end.
:::

1. On the **Publish** tab, click **Stop Voting**.
2. Click the channel, for example **Stop Online Voting**.
3. Confirm the action.
4. Do steps 1 to 3 for each other open channel.

**Expected result:** the message "Election status changed" shows. Voters cannot vote on the
closed channels.

## 7. Schedule the voting period (alternative)

Use scheduled events to open and close the voting period at a set time.

1. Open the election event.
2. Click the **Scheduled Events** tab.
3. Click **Create Scheduled Event**.
4. In **Type**, select **Start Voting Period**.
5. In **Election**, select an election. Leave it empty for all the elections.
6. Select the voting channels. **Online** and **Kiosk** are selected by default.
7. Type the date and the time in **Start Date and Time** (for an end event, **End Date and
   Time**). The admin portal uses the time zone of your computer. The label of the field shows
   the time zone.
8. Click the save button.
9. Do steps 3 to 8 again with the type **End Voting Period**.

**Expected result:** the **Scheduled Events** tab shows the two events.

An election has one scheduled event of each type. If you create the same type again for the same
election, the admin portal changes the date of the existing event. If a scheduled event cannot
run, for example because the initialization report is missing, it runs again at the next check.
See [Scheduled events](../01-tutorials/13-admin_portal_tutorials_scheduled-events.md).

:::caution CAUTION
Check the time zone in the label of the date field. If your computer is not in the time zone of
the election, convert the times.
:::

## If there is a problem

| Problem | Action |
| --- | --- |
| "Election event is locked down" | Set **Lockdown Status** to **Not Locked Down** on the **Data** tab. See [Create the Election Event](02-event.md). |
| "Ballot publication not generated yet, can't publish." | Make a publication first. See [step 1](#1-make-a-publication). |
| "Error change ballot publication status" | The status change is not permitted now. Check the current status of the channel, then try again. If you were asked for your password, confirm it first. |
| **Start Online Voting** is not available. | Make the initialization report, if the election needs one. If the channel is closed, contact Sequent support. |
| A channel is not in the menu, or is not available. | Allow the channel in **Voting Channels Allowed** on the **Data** tab. |
| "Ballot style generation failed: ..." | The ballot content is not valid. Read the message, correct the election event and make a new publication. |
| **Stop Online Voting** is not available. | A scheduled event does not permit the end of the voting period yet. Look at the **Scheduled Events** tab. |
| Voters do not see a contest. | Make sure that the contest is in the area of the voter. Then make and publish a new publication. |

## More information

- [Publish the ballot (with screenshots)](../01-tutorials/12-admin_portal_tutorials_publish-election.md)
- [Initialization report](../01-tutorials/11-admin_portal_tutorials_initialization-report.md)
- [Start, pause and stop the voting](../01-tutorials/14-admin_portal_tutorials_start-pause-stop-election_copy.md)
- [Scheduled events](../01-tutorials/13-admin_portal_tutorials_scheduled-events.md)
- [Election event: Publish](../02-reference/02-election-event/09-election_management_election-event_publish.md)
- [Election: Publish](../02-reference/03-election/05-election_management_election_publish.md)
- [Election event: Scheduled Events](../02-reference/02-election-event/12-election_management_election-event_scheduled-events.md)

**Next:** [Run the Tally Ceremony](05-tally.md).
