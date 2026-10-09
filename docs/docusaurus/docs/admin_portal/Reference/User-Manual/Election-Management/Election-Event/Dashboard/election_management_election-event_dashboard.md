---
id: election_management_election_event_dashboard
title: Dashboard
description: "The Dashboard tab shows the stage of the election event and its main numbers. Use it to follow the election event from its creation to the results. The numbers update automatically."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Dashboard** tab shows the stage of the election event and its main numbers. Use it to
follow the election event from its creation to the results. The numbers update automatically.

## Open the tab

1. In the menu on the left, click the election event.
2. Click the **Dashboard** tab.

The tab shows only if your role has the **Admin Dashboard View** permission.

## Stages

The top of the tab shows the stages of the election event. The admin portal marks the last
stage that is complete.

| Stage | The stage is complete when |
| --- | --- |
| **Created** | The election event exists. |
| **Keys** | A key ceremony has the status `SUCCESS`. |
| **Publish** | The ballot is published. |
| **Started** | The voting period is open or paused. |
| **Ended** | The voting period is closed. |
| **Results** | A tally of the type **Electoral Results** is complete. An initialization report does not count. |

## Numbers and charts

| Item | Content |
| --- | --- |
| **Eligible Voters** | The number of voters of the election event. |
| **Elections** | The number of elections. |
| **Areas** | The number of areas. |
| **Emails sent** | The number of emails that the platform sent for the election event. |
| **SMS sent** | The number of text messages that the platform sent. |
| **Votes by day** | A chart of the votes cast in the last seven days. |
| **Voters by channel** | A chart of the voters for each channel. In version 9.0 only the online channel has values. |
| **IP Addresses** | A table of the IP addresses from which voters voted: **IP**, **Country**, **Vote Count**, **Election Name**. It shows only with the **View IP Address** permission. |

## Links to the voting portal

The bottom of the tab has these links:

| Link | Use |
| --- | --- |
| **Voter Login URL** | The sign-in page of the voting portal for this election event. |
| **Voter Enroll URL** | The enrollment page, for election events where voters enroll. |
| **Voter Enroll Kiosk URL** | The enrollment page for kiosks. It shows only if the **Kiosk** voting channel is on for the election event. |

## If there is a problem

| Problem | Action |
| --- | --- |
| The tab is not there. | Your role does not have the **Admin Dashboard View** permission. Ask your tenant administrator. |
| A number shows `-`. | The platform has no data for this number yet. |
| **Eligible Voters** is not the number that you expect. | Check the voter list. See [Add the Voters](../../../../../procedures/03-voters.md). |
| The **Keys** stage is not marked after the key ceremony. | The key ceremony is not complete. See [Run the Key Ceremony](../../../../../procedures/04-keys.md). |

## Related pages

- [Election: Dashboard](../../Election/Dashboard/election_management_election_dashboard.md)
- [Monitoring](../Monitoring/election_management_election-event_monitoring.md)
