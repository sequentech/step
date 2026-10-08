---
id: election_management_election_event_monitoring
title: Monitoring
description: "The Monitoring tab shows the progress of the election event during the voting period and the tally. It shows counts and percentages for the voters, the elections and the tally."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Monitoring** tab shows the progress of the election event during the voting period and
the tally. It shows counts and percentages for the voters, the elections and the tally. The
numbers update automatically.

## Open the tab

1. In the menu on the left, click the election event.
2. Click the **Monitoring** tab.

The tab shows only if your role has the **Election Event Monitoring Dashboard View**
permission. Each item of the tab also needs its own **Read Monitoring ...** permission, so your
tab can show fewer items than the tables below.

## Sections

In the items of the **Polls** and **Tally** sections, the word "Posts" means the elections of
the election event.

### Voters

| Item | Content |
| --- | --- |
| **Enrolled Overseas Voters** | The voters who enrolled. |
| **Approval Status: Approved/Disapproved Voters** | The enrollment applications that were approved and rejected. |
| **Manually Approved/Disapproved Voters** | The applications that an administrator approved or rejected on the **Approvals** tab. |
| **Automatically Approved/Disapproved Voters** | The applications that the platform approved or rejected automatically. |
| **Authenticated Voters** | The voters who signed in, with the counts of **Invalid User Errors:** and **Invalid Password Errors:**. |

### Polls

| Item | Content |
| --- | --- |
| **Posts with Initialized Systems** | The elections with an initialization report. |
| **Posts with Online Voting Opened** | The elections where online voting is open. |
| **Posts with Both Voting Channels Closed** | The elections where online and kiosk voting are closed. |
| **Posts with Voting Started** | The elections where voting has started. |
| **Voter Turnout** | The voters who voted, and their percentage of the eligible voters. |

### Tally

| Item | Content |
| --- | --- |
| **Posts with Active Vote Counting** | The elections with a tally in progress or complete. |
| **Posts with Generated ERs** | The elections with a complete tally. |
| **Posts with Transmitted Results** | The elections with transmitted results. |

### Testing

| Item | Content |
| --- | --- |
| **Test Election Voter Count** | The voters who voted in a test election. |

## If there is a problem

| Problem | Action |
| --- | --- |
| The tab is not there. | Your role does not have the **Election Event Monitoring Dashboard View** permission. Ask your tenant administrator. |
| A section or an item is not there. | Your role does not have the **Read Monitoring ...** permission of that item. |
| The enrollment items show `0`. | The election event does not use enrollment. This is correct. |

## Related pages

- [Election: Monitoring](../../Election/Monitoring/election_management_election_monitoring.md)
- [Dashboard](../Dashboard/election_management_election-event_dashboard.md)
- [Approvals](../Approvals/election_management_election-event_approvals.md)
