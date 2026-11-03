---
id: election_management_election_data
title: Data
description: "The Data tab of an election holds the configuration of one election: its names, languages, voting channels, ballot design and policies."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Data** tab of an election holds the configuration of one election: its names, languages,
voting channels, ballot design and policies. It also holds the policies that control the
initialization report and the tally.

## Open the tab

1. In the menu on the left, open the election event.
2. Click the election.
3. Click the **Data** tab.

The tab shows with the **View Election Data** permission. To save changes, your role needs the
**Edit Election** permission.

To create an election, click the arrow next to the election event in the menu to show its elections, then click
**Create an Election**. The form has **Name**, **External ID** and **Description**. See
[Create the Election Event](../../../../../procedures/02-event.md#3-create-the-elections).

## Sections

| Section | Fields | Use |
| --- | --- | --- |
| **General** | **External ID**, and one tab for each language with **Name**, **Alias** and **Description** | The identifier and the texts of the election. You cannot change the **External ID** after you save it. |
| **Language** | One check box for each language, and a **Default** choice | The languages of the election. |
| **Voting Channels Allowed** | **Online**, **Kiosk** | The voting channels of the election. |
| **Ballot Design** | **Audit Button Display Options**, the order of the contests, **Reorder contests** | The look of the ballot. **Audit Button Display Options** has **Show**, **Not Show** and **Show In Help Dialog**. The contest order has **Random**, **Custom** and **Alphabetical**. |
| **Image** | An image file | The image of the election in the voting portal. |
| **Advanced Configuration** | See the table below | The policies of the election. |

### Advanced Configuration

| Field | Values | Meaning |
| --- | --- | --- |
| **Cast Vote Confirmation Modal** | On or off | If the voting portal shows a confirmation window when the voter casts the vote. |
| **Number of allowed votes** | A number | The number of votes that a voter can cast in this election. |
| **Gold level Authentication Policy** | **Gold level Authentication**, **No Gold level Authentication** | The authentication level that the voter needs to cast a vote. The default is **No Gold level Authentication**. |
| **Permission Label** | Text | The permission label of the election. It shows only with the **Edit Permission Label** permission. See [Permission Labels](../../../../../Tutorials/admin_portal_tutorials_permission-labels.md). |
| Configuration file | A JSON file | Advanced settings of the election in JSON format. Change it only with the help of Sequent support. |
| **Initialize Report Policy** | **Not Required**, **Required** | If **Required**, **Start Voting** is not available until the initialization report exists. |
| **Grace Period Policy** (under **Custom filters**) | **No grace period**, **Grace period without alert** | The grace period at the end of the voting period. The default is **No grace period**. |
| **Grace period in seconds** (under **Custom filters**) | A number | The length of the grace period. The default is 0. |
| **Allow Tally** (under **Custom filters**) | **Allowed**, **Disallowed**, **Requires Voting Period End** | When the tally can start. The default is **Allowed**. |

| **Allow Tally** value | Meaning |
| --- | --- |
| **Allowed** | You can start the tally at any time. |
| **Disallowed** | You cannot start the tally. |
| **Requires Voting Period End** | You can start the tally only after the voting period is closed. |

:::danger WARNING
With **Allowed**, the admin portal lets you start the tally while voters can still vote. Use
**Requires Voting Period End** if the tally must wait for the end of the voting period.
:::

## If there is a problem

| Problem | Action |
| --- | --- |
| **External ID** cannot be changed. | This is correct. The **External ID** is fixed after you save it. |
| **Permission Label** is not in the form. | Your role does not have the **Edit Permission Label** permission. |
| "You don't have permission to access this election." | The election has a permission label that your account does not have. |
| **Start Voting** is not available. | **Initialize Report Policy** is **Required**. Make the initialization report. See [Publish and Manage the Voting Period](../../../../../procedures/05-publish.md#4-make-the-initialization-report-if-necessary). |

## Related pages

- [Create the Election Event](../../../../../procedures/02-event.md#3-create-the-elections)
- [Election event: Data](../../Election-Event/Data/election_management_election-event_data.md)
- [Election event: Tally](../../Election-Event/Tally/election_management_election-event_tally.md)
- [Election event: Scheduled Events](../../Election-Event/Scheduled-Events/election_management_election-event_scheduled-events.md)
