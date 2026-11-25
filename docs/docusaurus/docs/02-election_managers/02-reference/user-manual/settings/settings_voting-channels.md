---
id: settings_voting-channels
title: Voting Channels
description: "The VOTING CHANELS tab is in Settings. It sets the tenant-level voting channels: online, kiosk and telephone. The tenant administrator uses it."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **VOTING CHANELS** tab is in **Settings**. It sets the tenant-level voting channels:
online, kiosk and telephone. The tenant administrator uses it.

:::note
In version 10.0, a new election event copies the values of this tab when you create it. After
that, the election event uses its own values. Change them on the **Data** tab of the election
event, in the section **Voting Channels Allowed**. A change on this tab does not change the
election events that exist. See
[Election event data](../../02-election-event/03-election_management_election-event_data.md).
:::

## How to open it

1. In the menu on the left, click **Settings**.
2. Click the **VOTING CHANELS** tab. The screen shows the label as written here.

## Required permission

To open **Settings**, your account must have the **Edit Tenant** and **View settings**
permissions. If not, the screen shows "You don't have permission to access settings."

## Fields

| Switch | Meaning | Default |
| --- | --- | --- |
| **Online Voting** | Voters vote on the internet in the voting portal. | On |
| **Kiosk Voting** | Voters vote on a kiosk device. | Off |
| **Telephone Voting** | Voters vote by telephone. | Off |

There is no **Save** button. When you click a switch, the admin portal saves the change
immediately.

## Change a voting channel

1. Open the **VOTING CHANELS** tab.
2. Click the switch of the voting channel.

**Expected result:** the switch shows the new position.

## If there is a problem

| Problem | Action |
| --- | --- |
| The message "You don't have permission to access settings." shows. | Ask a tenant administrator or Sequent support to give your account the necessary permissions. |
| An election event does not accept a voting channel. | Open the **Data** tab of the election event and check **Voting Channels Allowed**. |
| A new election event does not have the channels that you expect. | The election event copied this tab when you created it. Turn on the channels in **Voting Channels Allowed** on the **Data** tab of the election event. |

**Related procedure:** [Set Up the Tenant](../../../03-procedures/01-tenant.md).
