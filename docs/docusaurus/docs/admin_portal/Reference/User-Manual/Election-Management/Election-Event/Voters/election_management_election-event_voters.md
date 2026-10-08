---
id: election_management_election_event_voters
title: Voters
description: "The Voters tab is the voter list of the election event. Use it to import, add, change, delete and export voters, and to send messages to them. For the full procedure, see Add the Voters."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Voters** tab is the voter list of the election event. Use it to import, add, change,
delete and export voters, and to send messages to them. For the full procedure, see
[Add the Voters](../../../../../procedures/03-voters.md).

## Open the tab

1. In the menu on the left, click the election event.
2. Click the **Voters** tab.

The tab shows with the **View Election Event Voters** permission. Each button needs its own
permission, as the tables below show.

## Toolbar

| Button | Use | Permission |
| --- | --- | --- |
| **Columns** | Selects the columns of the list. | **View Election Event Voters Columns** |
| **Add filter** | Adds a filter, for example **Area**, **Enabled** or **Voted**. | **View Election Event Voters Filters** |
| **Custom Filters** | Applies a saved filter. It shows only if the election event has custom filters on its **Data** tab. | **View Election Event Voters Filters** |
| **Add** | Opens the form to create a voter. | **Create Voter** |
| **Import** | Opens the **Import Voters** panel. | **Import Voter** |
| **Export** | Exports the voter list. The task "Export Voters" makes the file. | **Export Voter** |
| **Send** | Opens the **Send Notification** form for all voters. | **Send Notification** |

When the list is empty, the tab shows "No voters yet." with the buttons **Create Voter** and
**Import**.

## Columns

| Column | Content |
| --- | --- |
| **Id** | The identifier of the voter. |
| **Email Verified** | If the email address of the voter is verified. |
| **Enabled** | If the voter can sign in. |
| Profile fields | For example **Username**, **Email**, **First Name** and **Last Name**. The fields depend on the user profile of the election event. |
| **Area** | The area of the voter. `-` means that the voter has no area. |
| **Voted** | If the voter has cast a vote. |
| **Actions** | The actions menu of the voter. |

## Actions of a voter

| Action | Use | Permission |
| --- | --- | --- |
| **Send** | Sends a message to this voter. | **Send Notification** |
| **Edit** | Changes the data of the voter. | **Edit Voter**, or **Edit voters email/phone fields** |
| **Delete** | Deletes the voter, after the question "Are you sure you want to delete this voter?". | **Delete Voter** |
| **Manually Verify** | Makes a PDF with a QR code link. The voter uses it to sign in without the online identity check. The voter needs an email address or a phone number. | **Manually Verify Voter** |
| **Change password** | Sets a new password for the voter. | **Change Voter Password** |
| **User's Logs** | Shows the log entries of this voter. | **View Election Event Voters Logs** |

To act on more voters, select them in the list. Then click **Send** or **Delete** above the
list.

## Voter form

| Field | Use |
| --- | --- |
| Profile fields | The fields of the user profile, for example **Username**. **Username** is always required. |
| **Enabled \*** | If the voter can sign in. |
| **Area \*** | The area of the voter. Type three or more letters to search. |
| **Password:** and **Repeat Password:** | The password of the voter. |
| **Temporary** | If on, the voter must change the password at the next sign-in. It is on by default. |
| **Save** | Saves the voter. |

## Import file

The import file is a CSV file with a header row. The columns are in
[Add the Voters](../../../../../procedures/03-voters.md#1-prepare-the-voter-file). The import
panel also has **Integrity Check (SHA-256)**. The import runs as the task **Import Users** on
the **Tasks** tab.

## Send Notification form

| Field | Use |
| --- | --- |
| **Audience** | **Everyone**, **Those who didn't vote yet**, **Those who already voted**, or the selected voters. |
| **Schedule** | **Send now**, or "Date and time to start sending notifications". |
| **Languages** | The languages of the message. |
| **Communication Template** | **Template Method** (**Email** or **SMS**), **Communication Type** and **Template Alias**. |
| **Send Notification** | Sends or schedules the message. |

## If there is a problem

| Problem | Action |
| --- | --- |
| "Voters Import Scheduled Successfully" shows, but no voters show. | The import runs as a task. Open the **Tasks** tab and check the task **Import Users**. |
| "Error importing Voters" | The import did not start. Try again. |
| "This voter can not be manually verified because they do not have an email address or phone number attributed to them." | Add an email address or a phone number to the voter. |
| "Error editing voter" | Check the fields, then save again. |
| "Error sending the notification: ..." | Read the error. Check that a template of the selected type and method exists. |
| A button is not there. | Your role does not have its permission. Ask your tenant administrator. |

## Related pages

- [Add the Voters](../../../../../procedures/03-voters.md)
- [Election: Voters](../../Election/Voters/election_management_election_voters.md)
- [Areas](../Areas/election_management_election-event_areas.md)
- [Approvals](../Approvals/election_management_election-event_approvals.md)
- [Templates](../../../Templates/admin_portal_reference_user-manual_templates.md)
