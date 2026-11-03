---
id: admin_portal_reference_user_manual_settings_voting_channels
title: Voting Channels
description: "The VOTING CHANELS tab sets the voting channels that the tenant can use. The tab name has a spelling error in version 9.0."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **VOTING CHANELS** tab sets the voting channels that the tenant can use. The tab name has
a spelling error in version 9.0. Each election event and each election then selects its own
channels in **Voting Channels Allowed** on its **Data** tab.

## Open the tab

1. In the menu on the left, click **Settings**.
2. Click the **VOTING CHANELS** tab.

## Fields

| Field | Use |
| --- | --- |
| **Online Voting** | Voters vote in the voting portal. |
| **Kiosk Voting** | Voters vote at a voting kiosk. |

The admin portal saves a change immediately when you click a switch. There is no **Save**
button.

## If there is a problem

| Problem | Action |
| --- | --- |
| **Online Voting** shows as on after you turn it off. | This is a display error of version 9.0: the switch always shows as on. Ask Sequent support to confirm the saved value. |
| A channel is not in **Voting Channels Allowed** of an election event. | Turn on the channel here, then open the election event again. |

## Related pages

- [Settings](../admin_portal_reference_user-manual_settings.md)
- [Election event: Data](../../Election-Management/Election-Event/Data/election_management_election-event_data.md)
