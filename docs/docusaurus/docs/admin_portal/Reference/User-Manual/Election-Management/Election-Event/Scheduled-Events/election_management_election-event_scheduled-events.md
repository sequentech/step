---
id: election_management_election_event_scheduled_events
title: Scheduled Events
description: "The Scheduled Events tab makes the platform do an action at a set date and time, for example open or close the voting period."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Scheduled Events** tab makes the platform do an action at a set date and time, for
example open or close the voting period. The header of the tab says "Manages the configuration
of the automatic execution of events like the start or end of the voting period."

## Open the tab

1. In the menu on the left, click the election event.
2. Click the **Scheduled Events** tab.

The tab shows with the **View Election Event Scheduled** permission. It is hidden when the
election event is locked down. The actions need these permissions:

| Action | Permission |
| --- | --- |
| Create | **Create Scheduled Event** |
| Change | **Edit Scheduled Events** |
| Delete | **Delete Scheduled Event** |
| Select the columns | **Election Event Scheduled Event Columns** |

## List

When the list is empty, the tab shows "No Scheduled Events yet." and **Create Scheduled
Event**. When there are events, the toolbar has **Add**.

| Column | Content |
| --- | --- |
| **Election** | The election of the event. |
| **Type** | The type of the event. |
| **Stopped At** | When the event was stopped, if it was stopped. |
| **Scheduled At** | The date and time of the event. |
| **Actions** | The edit icon and the delete icon. |

## Event types

| Type | Action at the date and time | **Election** |
| --- | --- | --- |
| **Start Voting Period** | Opens the voting period. | Optional. Empty means all the elections. |
| **End Voting Period** | Closes the voting period. | Optional. Empty means all the elections. |
| **Allow Initialization Report** | Permits the initialization report of the election. | Required |
| **Allow Voting Period End** | Permits the end of the voting period of the election. **Stop Voting** is not available while the end is not permitted. | Required |
| **Allow Tally** | Sets **Allow Tally** of the election to **Allowed**. | Required |
| **Start Enrollment Period** | Opens the enrollment of voters. | Not used |
| **End Enrollment Period** | Closes the enrollment of voters. | Not used |
| **Start Lockdown Period** | Locks down the election event. | Not used |
| **End Lockdown Period** | Ends the lockdown. | Not used |

## Create a scheduled event

1. Click **Add**. If the list is empty, click **Create Scheduled Event**.
2. In **Type**, select the type.
3. In **Election**, select the election, if the type uses one.
4. Type the date and time. The label of the field shows your time zone, for example
   "Start Date and Time (Europe/Madrid)".
5. Click **Save**.

**Expected result:** the event shows in the list.

:::caution CAUTION
The admin portal uses the time zone of your computer. If your computer is not in the time zone
of the election, convert the times before you type them.
:::

You cannot change the **Type** or the **Election** of an existing event. To change them,
delete the event and create a new one.

## If there is a problem

| Problem | Action |
| --- | --- |
| "Error creating Scheduled Event" | Check the type, the election and the date, then save again. |
| "Error editing Scheduled Event" | Check the date, then save again. |
| The voting period did not open at the time. | Check the time zone of the date. Check that the ballot is published and that the initialization report exists, if it is required. |
| **Stop Voting** is not available on the **Publish** tab. | The end of the voting period is not permitted yet. Check the **Allow Voting Period End** event of the election. |

## Related pages

- [Publish and Manage the Voting Period](../../../../../procedures/05-publish.md#7-schedule-the-voting-period-alternative)
- [Publish](../Publish/election_management_election-event_publish.md)
- [Setting Up An Automated Election](../../../../../Tutorials/admin_portal_tutorials_setting-up-an-automated-election.md)
