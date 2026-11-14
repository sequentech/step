---
id: election_management_election_dashboard
title: Dashboard
description: "The Dashboard tab of an election shows the main numbers of one election. The numbers update automatically."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Dashboard** tab of an election shows the main numbers of one election. The numbers update
automatically.

## Open the tab

1. In the menu on the left, open the election event.
2. Click the election.
3. Click the **Dashboard** tab.

The tab shows only if your role has the **View Election Dashboard** permission.

## Numbers and charts

| Item | Content |
| --- | --- |
| **Eligible Voters** | The number of voters who can vote in this election. |
| **Actual Voters** | The number of voters who have cast a vote in this election. |
| **Areas** | The number of areas. |
| **Emails sent** | The number of emails that the platform sent for this election. |
| **SMS sent** | The number of text messages that the platform sent for this election. |
| **Votes by day** | A chart of the votes cast in the last seven days. |
| **Voters by channel** | A chart of the voters for each channel. In version 9.0 only the online channel has values. |
| **IP Addresses** | A table of the IP addresses from which voters voted in this election. It shows only with the **View Election IP Address** permission. |

Use **Actual Voters** to check the result of the tally: after the tally, compare it with
**Total Votes Counted**. See [Get the Results](../../../../../procedures/07-results.md).

## If there is a problem

| Problem | Action |
| --- | --- |
| The tab is not there. | Your role does not have the **View Election Dashboard** permission. Ask your tenant administrator. |
| "You don't have permission to access this election." | The election has a permission label that your account does not have. See [Permission Labels](../../../../../Tutorials/admin_portal_tutorials_permission-labels.md). |
| A number shows `-`. | The platform has no data for this number yet. |

## Related pages

- [Election event: Dashboard](../../Election-Event/Dashboard/election_management_election-event_dashboard.md)
- [Election: Monitoring](../Monitoring/election_management_election_monitoring.md)
