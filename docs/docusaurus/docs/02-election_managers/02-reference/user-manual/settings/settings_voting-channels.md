---
id: settings_voting-channels
title: Voting Channels
description: "The VOTING CHANELS tab is in Settings. It keeps a tenant-level record of the voting channels: online, kiosk and telephone. The tenant administrator uses it."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **VOTING CHANELS** tab is in **Settings**. It keeps a tenant-level record of the voting
channels: online, kiosk and telephone. The tenant administrator uses it.

:::note
In this version, no other screen reads the values of this tab. The voting channels that an
election event accepts are set on the **Data** tab of the election event, in the section
**Voting Channels Allowed**. See
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
| **Online Voting** shows on after you turned it off and opened the tab again. | The tab always shows **Online Voting** on when it loads. This has no effect on the election events. Set the voting channels of each election event on its **Data** tab. |
| An election event does not accept a voting channel. | Open the **Data** tab of the election event and check **Voting Channels Allowed**. |

**Related procedure:** [Set Up the Tenant](../../../03-procedures/01-tenant.md).
