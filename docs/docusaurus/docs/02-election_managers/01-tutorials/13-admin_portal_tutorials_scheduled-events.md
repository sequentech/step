---
id: admin_portal_tutorials_scheduled-events
title: Scheduled events
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The main path to schedule the voting period is in
[04-PUBLISH](../03-procedures/04-publish.md#7-schedule-the-voting-period-alternative). This page
gives all the types of scheduled events, their effect and the rules that apply when they run.

A scheduled event makes a change in the election event at a set date and time, with no action
from a person. Scheduled events are on the **Scheduled Events** tab of the election event.

## Permissions

| Permission | Lets you |
| --- | --- |
| **View Election Event Scheduled** | See the **Scheduled Events** tab. |
| **Create Scheduled Event** and **Edit Scheduled Events** | Create a scheduled event. |
| **Edit Scheduled Events** | Change the date of a scheduled event. |
| **Delete Scheduled Event** | Delete a scheduled event. |

## Types of scheduled events

| **Type** | **Election** field | Effect when the event runs |
| --- | --- | --- |
| **Start Voting Period** | Optional | Opens the selected voting channels. |
| **End Voting Period** | Optional | Closes the selected voting channels. |
| **Allow Initialization Report** | Required | Sets the status of the election so that initialization reports are permitted. See [Initialization report](11-admin_portal_tutorials_initialization-report.md). |
| **Allow Voting Period End** | Required | Permits the end of the voting period of the election. **Stop Online Voting** becomes available if the configuration of the election did not permit it. |
| **Allow Tally** | Required | Sets **Allow Tally** of the election to **Allowed**. |
| **Start Enrollment Period** | Not shown | Lets voters register themselves, and sets **Enrollment** of the election event to **Enabled**. |
| **End Enrollment Period** | Not shown | Stops the registration of voters, and sets **Enrollment** to **Disabled**. |
| **Start Lockdown Period** | Not shown | Sets **Lockdown Status** of the election event to **Locked Down**. |
| **End Lockdown Period** | Not shown | Sets **Lockdown Status** to **Not Locked Down**. |

For **Start Voting Period** and **End Voting Period**, leave **Election** empty to apply the event
to all the elections of the election event.

## Create a scheduled event

1. Open the election event.
2. Click the **Scheduled Events** tab.
3. Click **Create Scheduled Event**.
4. In **Type**, select the type.
5. If the form shows **Election**, select the election.
6. For **Start Voting Period** or **End Voting Period**, select the channels in **Voting
   Channels**.
7. Type the date and the time.
8. Click the save button.

**Expected result:** the list shows the event with its **Scheduled At** date. **Stopped At**
shows "-".

:::note
After you save, the admin portal can show the message "Scheduled Event edited successfully",
also for a new event.
:::

## Voting channels

The **Voting Channels** field shows **Online**, **Kiosk**, **Early voting** and
**Telephone voting**. **Online** and **Kiosk** are selected by default. One channel must stay
selected.

- The event changes only the channels that are allowed in **Voting Channels Allowed** of each
  election. It ignores the other selected channels.
- A **Start Voting Period** event cannot select **Online** and **Early voting** together. The form
  shows "A start schedule cannot open Online and Early voting together: early voting has to start
  before online voting." and the save button is not available.
- To have early voting before online voting, make two **Start Voting Period** events. Make one for
  **Early voting** and one for **Online**, at a later time.

## Date, time and time zone

The admin portal uses the time zone of your computer. The label of the date field shows it, for
example **Start Date and Time (Europe/Madrid)**. For all types except **Start Voting Period**, the
label is **End Date and Time** with the time zone. The list shows the dates in the time zone of
your computer too.

:::caution CAUTION
Check the time zone in the label before you save. If your computer is not in the time zone of the
election, convert the times.
:::

## How a scheduled event runs

The platform checks the scheduled events at short intervals. When the date of an event is reached,
the platform runs it and fills **Stopped At** with the time. An event runs one time only. If you
save a date in the past, the event runs at the next check.

The voting period events follow these rules:

- **Start Voting Period** opens a channel only if its status is `NOT_STARTED` or `PAUSED`. It does
  not open a channel again after it is `CLOSED`.
- **End Voting Period** closes a channel only if its status is `OPEN` or `PAUSED`. It does not
  change a channel that never started.
- A start of early voting skips each election where online voting has already started.
- For an event with an election, the platform applies the same checks as the **Publish** tab of
  the election. For example, if the initialization report is required and missing, the election
  does not open.

:::caution CAUTION
After the date, look at the status of the voting channels on the **Publish** tab. If a check
fails, **Stopped At** stays empty and the platform tries the event again at each check. The change
can then occur later, when the cause is removed. Correct the cause, or delete the event and change
the status by hand. See
[Start, pause and stop the voting period](14-admin_portal_tutorials_start-pause-stop-election_copy.md).
:::

## One event of each type

Each election has one scheduled event of each type. The election event also has one event of each
type for the events with no election. If you create an event of a type that already exists for
the same election, the platform changes the date of the existing event.

## Change or delete a scheduled event

To change the date:

1. In the row of the event, click the edit icon.
2. Change the date and the time. You can also change the voting channels.
3. Click the save button.

You cannot change the **Type** or the **Election**. To change them, delete the event and create a
new one.

To delete an event:

1. In the row of the event, click the delete icon.
2. Read the message "Are you sure you want delete this Scheduled Event?".
3. Click **Delete**.

**Expected result:** the event goes out of the list. It does not run.

## Find an event

Use the filters of the list: **Type**, **Election** and the id of the event. The list has the
columns **Election**, **Type**, **Stopped At** and **Scheduled At**.

## If there is a problem

| Problem | Action |
| --- | --- |
| The save button is not available. | Make sure that the form has a type, a date and, if necessary, an election. Do not select **Online** and **Early voting** together for a start. |
| "Error creating Scheduled Event" | Read the message and correct the form. If the message continues, contact Sequent support. |
| "Error editing Scheduled Event" | The change or the deletion did not occur. Try again, then contact Sequent support. |
| The event has a **Stopped At** time, but the channel status did not change. | The channel was not allowed in the election, or its status did not permit the change. Change the status on the **Publish** tab. |
| The date has passed, but **Stopped At** is empty. | A check failed, for example a required initialization report is missing. Correct the cause, or delete the event. |
| The event ran at the wrong time. | Check the time zone of your computer and the label of the date field. |
| A channel is not affected by the event. | Allow the channel in **Voting Channels Allowed** of the election. |
| The **Scheduled Events** tab is not shown. | Ask for the permission **View Election Event Scheduled**. If the election event is locked down, the tab can be hidden. |
