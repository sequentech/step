---
id: admin_portal_tutorials_setting-up-an-automated-election
title: Setting Up An Automated Election
description: "Version 9.0 does not have automatic ceremonies. The key ceremony and the tally ceremony always need the trustees: each trustee downloads and checks a key fragment, and uploads it again for the tally."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Version 9.0 does not have automatic ceremonies. The key ceremony and the tally ceremony always
need the trustees: each trustee downloads and checks a key fragment, and uploads it again for
the tally. There is no option to run the ceremonies without the trustees.

What version 9.0 can do automatically is the voting period and some permissions, with
scheduled events. Use them to open and close the voting period at set times, without an
administrator at the screen.

## What you can schedule

| Scheduled event type | Action at the date and time |
| --- | --- |
| **Start Voting Period** | Opens the voting period. |
| **End Voting Period** | Closes the voting period. |
| **Allow Voting Period End** | Permits the end of the voting period of an election. |
| **Allow Initialization Report** | Permits the initialization report of an election. |
| **Allow Tally** | Sets **Allow Tally** of an election to **Allowed**. |
| **Start Enrollment Period** and **End Enrollment Period** | Open and close the enrollment of voters. |
| **Start Lockdown Period** and **End Lockdown Period** | Lock down the election event and end the lockdown. |

## Schedule the voting period

Before you start, the key ceremony is complete and the ballot is published. If an election
needs an initialization report, make it before the start time.

1. Open the election event.
2. Click the **Scheduled Events** tab.
3. Create a **Start Voting Period** event with the start date and time.
4. Create an **End Voting Period** event with the end date and time.
5. Check the time zone in the label of the date field.

For the steps, see
[Publish and Manage the Voting Period](../procedures/05-publish.md#7-schedule-the-voting-period-alternative).

**Expected result:** the voting period opens and closes at the set times. The **Dashboard** tab
shows the stages **Started** and then **Ended**.

:::caution CAUTION
In version 9.0 you cannot open the voting period again after it closes. Check the end date and
time before you save the event.
:::

## Related pages

- [Scheduled Events](../Reference/User-Manual/Election-Management/Election-Event/Scheduled-Events/election_management_election-event_scheduled-events.md)
- [Run the Key Ceremony](../procedures/04-keys.md)
- [Run the Tally Ceremony](../procedures/06-tally.md)
