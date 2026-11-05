---
id: election_management_election_event_scheduled_events
title: Scheduled Events
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


Schedule lifecycle actions at explicit local times and zones. Start with
[Timezones and schedules](./17-timezones-and-schedules.md) for conversion,
CSV imports and timezone-rule changes. For voting authorization and retained
signed close deadlines, read
[Scheduled openings and closings with signatures](./18-signed-scheduled-transitions.md).

### Adding a Scheduled Event

- Select **Add**.
- Select a **Scheduled Event Type**.
- Select an **Election**.  
  - If no Election is selected, the Scheduled Event will apply to the Election Event and all related Elections.
- For **Start Voting Period** or **End Voting Period**, select one or more **Voting Channels**: Online, Kiosk, Early voting, or Telephone voting. Online and Kiosk are selected by default.
  - All four channels are always available in the form. When the schedule runs, it changes only channels enabled for each targeted election.
  - Existing schedules with no channel selection (or an empty selection in the payload) use Online and Kiosk for both start and end. Editing or exporting/importing a schedule preserves its channel selection.
- Select the **local date and time** and **timezone**. Review the converted
  instant and any daylight-saving or signing-outcome notices before saving.

> **Note:**  
> - Start Date and Time for **Start Voting Period** is the time the voting period starts.
> - Start Date and Time for **Stop Voting Period** is the time the voting period ends.  



### Timezone settings

Configure at least one IANA timezone on the election event and choose its primary
zone from that list. Each Post can select a configured zone or inherit the
primary. The server checks these settings for saves and imports, including direct
Hasura writes. Alternative names such as `US/Eastern` are stored as their
canonical names (`America/New_York`); abbreviations such as `EST` are rejected.
A zone assigned to a Post cannot be removed until that Post selects another zone
or inherits the primary.

Older events without timezone settings use UTC. Existing legacy settings are not
rewritten by unrelated saves. If a concurrent event save or scheduled transition
holds the settings lock, a Post timezone change is refused with a retry message;
save again after that operation finishes. A refusal leaves the stored settings
unchanged.
