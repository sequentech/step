---
id: election_management_election_monitoring
title: Monitoring
description: "The Monitoring tab of an election shows the progress of one election during the voting period. The numbers update automatically."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Monitoring** tab of an election shows the progress of one election during the voting
period. The numbers update automatically.

## Open the tab

1. In the menu on the left, open the election event.
2. Click the election.
3. Click the **Monitoring** tab.

The tab shows only if your role has the **Election Monitoring Dashboard View** permission. Each
item also needs its own **Read Monitoring ...** permission.

## Sections

### Voters

| Item | Content |
| --- | --- |
| **Enrolled Overseas Voters** | The voters of this election who enrolled. |
| **Approval Status: Approved/Disapproved Voters** | The enrollment applications that were approved and rejected. |
| **Manually Approved/Disapproved Voters** | The applications that an administrator approved or rejected. |
| **Automatically Approved/Disapproved Voters** | The applications that the platform approved or rejected automatically. |
| **Authenticated Voters** | The voters who signed in, with the counts of **Invalid User Errors:** and **Invalid Password Errors:**. |

### Polls

| Item | Content |
| --- | --- |
| **Voter Turnout** | The voters who voted in this election, and their percentage of the eligible voters. |

## If there is a problem

| Problem | Action |
| --- | --- |
| The tab is not there. | Your role does not have the **Election Monitoring Dashboard View** permission. Ask your tenant administrator. |
| A section or an item is not there. | Your role does not have the **Read Monitoring ...** permission of that item. |

## Related pages

- [Election event: Monitoring](../../Election-Event/Monitoring/election_management_election-event_monitoring.md)
- [Election: Dashboard](../Dashboard/election_management_election_dashboard.md)
