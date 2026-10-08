---
id: election_management_election_event_tally
title: Tally
description: "The Tally tab holds the tally ceremonies and the initialization reports of the election event. A tally ceremony decrypts and counts the votes."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Tally** tab holds the tally ceremonies and the initialization reports of the election
event. A tally ceremony decrypts and counts the votes. The trustees upload their key fragments,
then the election administrator starts the tally and gets the results. For the full
procedures, see [Run the Tally Ceremony](../../../../../procedures/06-tally.md) and
[Get the Results](../../../../../procedures/07-results.md).

## Open the tab

1. In the menu on the left, click the election event.
2. Click the **Tally** tab.

The tab shows with the **View Election Event Tally** permission and one of **Read Tally** or
**Start Tally**. To create a tally, your role needs **Create Ceremony**. To manage it, your role
needs **Admin Ceremony**. To download results, your role needs **Export Ceremony**. The tab is
hidden when the election event is locked down.

## Buttons

| Button | Use |
| --- | --- |
| **Start Tally Ceremony** | Creates a tally of the type **Electoral Results**. |
| **Generate Initialization Report** | Creates a tally of the type **Initialization Results**. It shows that the ballot box is empty before the voting period. |

Both buttons are available only when a key ceremony has the status `SUCCESS` and the ballot is
published. If not, the tab shows one of these messages:

- "The Tally Ceremony cannot start until the Keys Ceremony has been successfully completed."
- "The Tally Ceremony cannot start until you create one publication in the Publish tab."

## List of tallies

| Column | Content |
| --- | --- |
| **Tally Type** | **Electoral Results** or **Initialization Results**. |
| **Permission Labels** | The permission labels of the elections of the tally. |
| **Trustees** | The trustees of the tally. |
| **Number Elections** | The number of elections in the tally. |
| **Status** | The status of the tally. The table below gives the values. |
| **Actions** | **View Tally Ceremony**, **Cancel Tally Ceremony**, and for trustees **Add Tally Key**. |

| Status | Meaning |
| --- | --- |
| `STARTED` | The tally is created. The trustees can upload their key fragments. |
| `CONNECTED` | Enough trustees have uploaded their key fragment. The administrator can start the tally. |
| `IN_PROGRESS` | The platform mixes, decrypts and counts the votes. |
| `SUCCESS` | The tally is complete. The results are available. |
| `CANCELLED` | The tally is cancelled. |

**Cancel Tally Ceremony** is available only before the tally starts. It asks "Are you sure you
want to cancel the tally?". Click **Cancel Tally** to confirm. You cannot undo a cancellation.

## Tally wizard (administrator)

The wizard has four steps: **Start**, **Ceremony**, **Tally** and **Results**.

| Step | Content |
| --- | --- |
| **Start** | **Elections to Tally**: the elections to count, with the **Selected** column. **Keys Ceremony**: the key ceremony of these elections. Click **Start Tally Ceremony**, then **Ok**. |
| **Ceremony** | **Trustees**: "Key fragment import status", with the number of trustees who imported the key and the number needed. When the status is `CONNECTED`, click **Start Tally**, then **Start Tally** again in the question. |
| **Tally** | **Elections Tally Progress**: each election with its **Status** (`WAITING`, `MIXING`, `DECRYPTING`, `SUCCESS` or `ERROR`) and **Progress**. **General Information** and **Logs**. |
| **Results** | **Results & Participation**: the results by election, contest and area, and the download menu. See [Get the Results](../../../../../procedures/07-results.md). |

**Start Tally** is available only when every selected election permits the tally. **Allow
Tally** of the election must be **Allowed**, or **Requires Voting Period End** with the voting
period closed. See [Election: Data](../../Election/Data/election_management_election_data.md).

## Upload a key fragment (trustee)

A trustee sees "You have been invited to participate in a Tally ceremony.". The trustee clicks
**Add Tally Key** in the row of the tally, reads **Elections to Tally**, and uploads the key
fragment in **Trustees process**. The message "Backup verified successfully." shows. The
trustee then clicks **Next**.

## Result downloads

On the **Results** step, the actions menu of the election event, an election, a contest or an
area has items such as "Export in PDF format - 'Election 1' results". The formats are **PDF**,
**HTML**, **JSON**, **TAR_GZ** and **RECEIPTS_PDF**. A format shows only when the platform made
that document.

## If there is a problem

| Problem | Action |
| --- | --- |
| "You cannot continue the ceremony because no elections are selected or the elections are not published." | Select one or more published elections. |
| "You cannot continue the ceremony because the tally session is not connected or the start of the ceremony is not allowed." | Wait for more trustees. Then check **Allow Tally** of each election and the voting period. |
| "Could not create Tally" | Make sure that the elections have the same key ceremony and are published. |
| "Your key was already restored." | The trustee has uploaded the fragment already. No action is necessary. |
| An election shows `ERROR`. | Contact Sequent support. Do not start a new tally. |

## Related pages

- [Run the Tally Ceremony](../../../../../procedures/06-tally.md)
- [Get the Results](../../../../../procedures/07-results.md)
- [Keys](../Keys/election_management_election-event_keys.md)
- [Contest: Tally Sheets](../../Contest/Tally-Sheets/election_management_contest_tally-sheets.md)
- [Reports](../Reports/election_management_election-event_reports.md)
