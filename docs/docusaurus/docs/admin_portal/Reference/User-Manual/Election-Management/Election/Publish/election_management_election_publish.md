---
id: election_management_election_publish
title: Publish
description: "The Publish tab of an election publishes the ballot and controls the voting period of one election only. Use it when the elections of an election event open or close at different times."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Publish** tab of an election publishes the ballot and controls the voting period of one
election only. Use it when the elections of an election event open or close at different times.
To act on all the elections together, use the **Publish** tab of the election event.

## Open the tab

1. In the menu on the left, open the election event.
2. Click the election.
3. Click the **Publish** tab.

The tab shows with the **View Election Publish** permission.

## Screen elements

The tab has the same buttons, list and pages as the **Publish** tab of the election event. See
[Election event: Publish](../../Election-Event/Publish/election_management_election-event_publish.md).

These differences apply:

| Element | Difference |
| --- | --- |
| **Publish Changes** | Makes a publication for this election. |
| **Start Voting**, **Pause Voting**, **Stop Voting**, **Stop Kiosk Voting** | Change the voting status of this election only. |
| **Start Voting** | Not available if this election requires an initialization report and the report does not exist yet. |

:::danger WARNING
In version 9.0 you cannot open the voting period of the election again after **Stop Voting**.
:::

## If there is a problem

| Problem | Action |
| --- | --- |
| **Start Voting** is not available. | Check **Initialize Report Policy** on the **Data** tab of the election. If it is **Required**, make the initialization report on the **Tally** tab of the election event. |
| **Stop Voting** is not available. | A scheduled event does not permit the end yet. Check the **Scheduled Events** tab of the election event. |
| "You don't have permission to access this election." | The election has a permission label that your account does not have. See [Permission Labels](../../../../../Tutorials/admin_portal_tutorials_permission-labels.md). |

## Related pages

- [Publish and Manage the Voting Period](../../../../../procedures/05-publish.md)
- [Election event: Publish](../../Election-Event/Publish/election_management_election-event_publish.md)
- [Election: Data](../Data/election_management_election_data.md)
