---
id: election_management_election_event_areas
title: Areas
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Represents geographical or organizational divisions within which elections are conducted. These can range from small precincts or wards to larger regions such as districts or states.

### Table Options

- **Columns**: Enable or disable specific columns in the table view.
- **Add Filter**: Create a text-based filter for the table by column.
- **Add**: Manually add a new Area.
- **Import**: Import new Areas using a CSV file.
- **Export**: Export existing Areas to a CSV file.
- **Upsert**: Update or insert Areas using a CSV file.

## Early Voting per Area

When adding or editing an Area, you can enable an option called **Early Voting** for that specific area. If enabled, voters assigned to that area will be allowed to vote as soon as the **EARLY_VOTING** channel is started for the Election Event.

- **Visibility**: The Early Voting option appears only if the **EARLY_VOTING** channel is allowed at the Election Event level (Admin Portal > Election Event > Data > Voting Channels Allowed).
- **Effect**: Enabling Early Voting on an area does not start voting by itself; voting becomes possible for those area voters when the Election Event’s **EARLY_VOTING** channel is started in the Publish tab.
- **Import/Export**: The Early Voting setting is supported in CSV Import and Upsert operations, so you can manage this policy at scale.

Note: Kiosk voting status is independent. The Online voting channel governs Early Voting availability (see Election Event > Data > Voting Channels Allowed for details).
