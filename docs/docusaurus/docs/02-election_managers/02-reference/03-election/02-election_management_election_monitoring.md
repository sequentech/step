---
id: election_management_election_monitoring
title: Monitoring
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Election monitoring

When an election event is set up for monitoring, each of its elections'
**Dashboard** tab shows the event's monitoring dashboards for that election.
In monitoring an election is called a **Post**. The dashboards, widgets and
themes are the event's: there is no separate configuration per election.

On an election's page:

- the **Post** is fixed to the election, and the Post selector is not shown;
- the **Region** selector does not apply, because a Post lies in one region;
- the **Country** selector remains, for dashboards that offer it;
- widgets that do not follow the Post selector, such as attack detections,
  still show the whole event.

Everything else works as on the event page: widget selectors, **View data**,
**Export** (which exports the figures for this Post), freshness and widget
states. A user who may configure monitoring can also edit dashboards and
widgets from here; a change applies to the whole event.

The tab needs `election-dashboard-tab` on the election and `monitoring-view`
to show monitoring; without `monitoring-view` it shows the
[standard election dashboard](./01-election_management_election_dashboard.md).
A user whose permission labels do not allow the election cannot see its
figures.

See [Monitoring](../02-election-event/02-election_management_election-event_monitoring.md)
for the full guide.
