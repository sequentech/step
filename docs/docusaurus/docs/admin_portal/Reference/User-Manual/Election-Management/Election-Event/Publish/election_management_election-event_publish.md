---
id: election_management_election_event_publish
title: Publish
description: "The Publish tab makes the ballot available in the voting portal and controls the voting period of the whole election event. Each publication is a version of the ballot."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Publish** tab makes the ballot available in the voting portal and controls the voting
period of the whole election event. Each publication is a version of the ballot. Voters see the
last published version. For the full procedure, see
[Publish and Manage the Voting Period](../../../../../procedures/05-publish.md).

## Open the tab

1. In the menu on the left, click the election event.
2. Click the **Publish** tab.

The tab shows with the **View Election Event Publish** permission. It is hidden when the
election event is locked down.

## Toolbar

| Button | Use | Permission |
| --- | --- | --- |
| **Start Voting** | Opens the voting period on the voting channels of the election event. | **Edit Election State** and **Start Voting** |
| **Pause Voting** | Stops the voting for a time. | **Edit Election State** and **Pause Voting** |
| **Stop Voting** | Closes the **Online** channel. | **Edit Election State** and **Stop Voting** |
| **Stop Kiosk Voting** | Closes the **Kiosk** channel. It shows only if the **Kiosk** channel is on. | **Edit Election State** |
| **Publish Changes** | Makes a new publication and opens **Changes to be Published**. | **Edit Publish** and **Publish Changes** |

When there is no publication, the tab shows "No Publication Yet." and **Generate Publication**.

### When the buttons are available

| Button | Not available when |
| --- | --- |
| **Start Voting** | The voting period is open or closed. The initialization report is required and does not exist yet. |
| **Pause Voting** | The voting period is not open. |
| **Stop Voting** | The voting period is not open or paused. A scheduled event does not permit the end yet. |
| **Stop Kiosk Voting** | Kiosk voting is not open or paused. |
| **Publish Changes** | The voting period is closed. |

:::danger WARNING
In version 9.0 you cannot open the voting period again after **Stop Voting**. You also cannot
publish changes after it.
:::

## Publish History

The list **Publish History** shows the publications. Each row has a view icon and a preview
icon. The view icon opens **View Publication**, which compares the **Previous Publication** with
the **Publication**.

## Changes to be Published

This page opens after you generate a publication.

| Element | Use |
| --- | --- |
| **Current** and **Changes to Publish** | The differences between the published ballot and the new one. |
| **Regenerate** | Makes the publication again from the current data. |
| **Export** | Exports the publication. The task "Export Ballot Publication" makes the file. |
| **Back** | Returns to the list. |
| **Preview** | Opens the preview window. Select the area in **Select Area for Preview**, then click **Preview** or **Copy link**. |
| **Publish Changes** | Publishes the new ballot. The message "Ballot published" shows. |

## Sensitive actions

Generation, publication and the voting status changes ask for confirmation. The window
**Confirm Action** shows, for example "You are about to start voting period. Are you sure you
want to continue?". If your session needs a new check, the admin portal asks for your password.
See [Before You Start](../../../../../00-before-you-start.md#sensitive-actions).

## If there is a problem

| Problem | Action |
| --- | --- |
| "Election event is locked down" | Set **Lockdown Status** to **Not Locked Down** on the **Data** tab. |
| "Insufficient privileges" | Confirm your identity with your password, then repeat the action. |
| "Ballot publication not generated yet, can't publish." | Generate a publication first. |
| "Error loading ballot publication" | The generation failed. Try again. If the error stays, contact Sequent support. |
| "Error publishing ballot publication" | The publication failed. Try again. |
| "Error change ballot publication status" | The voting status did not change. Check the **Scheduled Events** tab and the initialization report. |
| "Rendering all changes might make the page unresponsive. Are you sure you want to continue?" | The list of changes is long. Confirm to see all the changes. |

## Related pages

- [Publish and Manage the Voting Period](../../../../../procedures/05-publish.md)
- [Election: Publish](../../Election/Publish/election_management_election_publish.md)
- [Scheduled Events](../Scheduled-Events/election_management_election-event_scheduled-events.md)
- [Tally](../Tally/election_management_election-event_tally.md)
