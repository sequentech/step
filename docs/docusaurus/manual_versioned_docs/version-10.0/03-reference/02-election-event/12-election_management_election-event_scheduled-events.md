---
id: election_management_election_event_scheduled_events
title: Scheduled Events
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


Allows the automation of an Election Event by scheduling key actions to occur automatically at specified times during the event lifecycle.

### Adding a Scheduled Event

- Select **Add**.
- Select a **Scheduled Event Type**.
- Select an **Election**.  
  - If no Election is selected, the Scheduled Event will apply to the Election Event and all related Elections.
- For **Start Voting Period** or **End Voting Period**, select one or more **Voting Channels**: Online, Kiosk, Early voting, or Telephone voting. Online and Kiosk are selected by default.
  - All four channels are always available in the form. When the schedule runs, it changes only channels enabled for each targeted election.
  - Existing schedules with no channel selection (or an empty selection in the payload) use Online and Kiosk for both start and end. Editing or exporting/importing a schedule preserves its channel selection.
- Select the **starting date and time** for this event.
- Select the **starting date and time** for when this scheduled event is triggered.

> **Note:**  
> - Start Date and Time for **Start Voting Period** is the time the voting period starts.
> - Start Date and Time for **Stop Voting Period** is the time the voting period ends.  


