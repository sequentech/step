---
id: admin_portal_tutorials_start-pause-stop-election
title: Start/Pause/Stop Election
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The main path is in [Publish and Manage the Voting Period](../03-procedures/04-publish.md#5-open-the-voting-period). This
page gives the status model of the voting channels, the rules for each menu item and the edge
cases.

## Voting channels and their status

The voting period has a separate status for each voting channel: **Online**, **Kiosk**,
**Early voting** and **Telephone voting**. Each election has these four statuses. The election
event also has them.

| Status | Meaning |
| --- | --- |
| `NOT_STARTED` | The channel has not opened. This is the status at the start. |
| `OPEN` | Voters can vote on the channel. |
| `PAUSED` | The voting is stopped for a time. You can open the channel again. |
| `CLOSED` | The voting period of the channel has ended. |

A channel can change status as follows:

| From | To |
| --- | --- |
| `NOT_STARTED` | `OPEN` |
| `OPEN` | `PAUSED` or `CLOSED` |
| `PAUSED` | `OPEN` or `CLOSED` |
| `CLOSED` | Not from the admin portal |

:::danger WARNING
Close a channel only at the official end of its voting period. The admin portal does not let you
open a closed channel again.
:::

## The two Publish tabs

| Tab | Effect of **Start Voting** and **Stop Voting** |
| --- | --- |
| **Publish** tab of the election event | Changes the channel of the election event and of all its elections. |
| **Publish** tab of an election | Changes the channel of that election only. |

The checks of the policies of an election apply only on the **Publish** tab of the election:

- **Initialize Report Policy** = **Required**: the election cannot open until its initialization
  report is complete. See [Initialization report](11-admin_portal_tutorials_initialization-report.md).
- The end of the voting period is not permitted: the election cannot close until an **Allow
  Voting Period End** scheduled event runs. See
  [Scheduled events](13-admin_portal_tutorials_scheduled-events.md).

:::caution CAUTION
If an election uses one of these policies, open and close it from the **Publish** tab of the
election. The **Publish** tab of the election event does not apply these checks.
:::

## When a menu item is available

**Start Voting**, **Pause Voting** and **Stop Voting** each open a menu with one item for each
channel, for example **Start Online Voting**.

| Menu item | Available when |
| --- | --- |
| **Start ... Voting** | The channel is allowed and its status is `NOT_STARTED` or `PAUSED`. |
| **Pause ... Voting** | The channel is allowed and its status is `OPEN`. |
| **Stop ... Voting** | The channel is allowed and its status is `OPEN` or `PAUSED`. |

A channel is allowed when it is selected in **Voting Channels Allowed** on the **Data** tab. Two
more rules apply on the **Publish** tab of an election:

- **Start Online Voting** is not available if the initialization report is required and is not
  complete.
- **Stop Online Voting** is not available if the configuration of the election does not permit
  the end of the voting period.

A button is not available when no item of its menu is available. The buttons are also not
available while a status change is in progress or while a publication is generated.

## Early voting and online voting

Early voting is a period before online voting. These rules apply:

- Open **Early Voting** before **Online Voting**.
- You cannot start early voting after online voting has started, also if online voting is now
  paused or closed. The platform refuses the change.
- When online voting opens or closes, the platform closes early voting automatically, if early
  voting has started.

## Confirm your identity

A change of the voting status is a sensitive action. When you click a menu item, the dialog
**Confirm Action** shows:

- If you signed in less than 60 seconds ago with a strong sign-in, the message is "You are about
  to start voting period. Are you sure you want to continue?". The text says pause or stop for
  the other actions. Click **Confirm**.
- In all other cases, the message asks for your password. Click **Confirm**, then sign in again.
  The admin portal then does the action that you selected.

See [Before You Start](../00-before-you-start.md#sensitive-actions).

## Permissions

To see the buttons, your account needs **Edit Election State** and the permission of the button:
**Start Voting**, **Pause Voting** or **Stop Voting**.

## What the platform records

Each change of status is recorded in the electoral log of the election event, with the channel and
the elections. See [Election logs](19-admin_portal_tutorials_election-logs.md).

## Automatic changes

Scheduled events can open and close channels at a set time. They never open a closed channel.
See [Scheduled events](13-admin_portal_tutorials_scheduled-events.md).

## If there is a problem

| Problem | Action |
| --- | --- |
| "Error change ballot publication status" | The platform refused the change. Look at the status of the channel and at the rules on this page. Sign in again, then try again. |
| **Start Online Voting** is not available. | Make sure that the channel is allowed and not closed. If the initialization report is required, make it. |
| **Stop Online Voting** is not available. | The end of the voting period is not permitted yet. Look at the **Scheduled Events** tab for an **Allow Voting Period End** event. |
| **Start Early Voting** fails. | Online voting has already started. You cannot open early voting now. |
| A closed channel must open again. | Contact Sequent support. The admin portal cannot do this. |
| All the buttons are not available. | Wait until the status change or the publication in progress is complete. Make sure that a channel is allowed. |
| The buttons are not shown. | Ask for the permissions **Edit Election State**, **Start Voting**, **Pause Voting** and **Stop Voting**. |
