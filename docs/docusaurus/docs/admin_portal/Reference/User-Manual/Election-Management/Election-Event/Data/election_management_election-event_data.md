---
id: election_management_election_event_data
title: Data
description: "The Data tab holds the configuration of the election event: its names, languages, voting channels, the look of the voting portal and the policies."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Data** tab holds the configuration of the election event: its names, languages, voting
channels, the look of the voting portal and the policies. It also has the export of the
election event and the import of candidates.

## Open the tab

1. In the menu on the left, click the election event.
2. Click the **Data** tab.

The tab shows only if your role has the **View Election Event Data** permission. To save
changes, your role needs the **Edit Election Event** permission. The tab is hidden when
**Lockdown Status** is **Locked Down**.

## Sections

The tab has these sections. Click a section to open it. Click **Save** at the bottom to keep
your changes.

| Section | Fields | Use |
| --- | --- | --- |
| **General** | One tab for each language, with **Name**, **Alias** and **Description** | The texts of the election event in the voting portal. |
| **Language** | One check box for each language, and a **Default** choice | The languages of the election event. Only the languages turned on in **Settings** > **LANGUAGES** show. |
| **Ballot Design** | **Skip Election List Screen**, **Show User Profile**, the order of the elections, **Reorder elections**, **Logo URL**, **Redirect Finish URL**, **Custom CSS** | The look and the behavior of the voting portal. Select the custom order to show **Reorder elections**. |
| **Voting Channels Allowed** | **Online**, **Kiosk** | The voting channels of the election event. |
| **Custom URLs Prefix** | **Login**, **Enrollment** and a third prefix for the external sign-in | Short addresses for the sign-in and enrollment pages. |
| **Support Materials** | **Support Materials Activated**, **Title** and **Subtitle** for each language, and the list of materials | Documents that voters can open in the voting portal. |
| **Advanced Configurations** | **Contest encryption policy**, **Lockdown Status**, **Voting Portal Countdown policy** with its two times in seconds, **Voter Signing Policy**, **Custom filters**, **Voter Authentication** | The policies of the election event. The table below gives the values. |

### Advanced Configurations

| Field | Values | Meaning |
| --- | --- | --- |
| **Contest encryption policy** | **Single Contest**, **Multiple Contests** | How the voting portal encrypts the contests of a ballot. The default is **Single Contest**. |
| **Lockdown Status** | **Not Locked Down**, **Locked Down** | **Locked Down** hides most tabs and stops the generation of publications. The default is **Not Locked Down**. |
| **Voting Portal Countdown policy** | **No Countdown**, **Countdown**, **Countdown with alert** | If the voting portal shows a countdown before the session of the voter ends. The two time fields are available only with a countdown. |
| **Voter Signing Policy** | **No signature**, **With signature** | The signature policy for the ballots that voters cast. The default is **No signature**. |
| **Custom filters** | A JSON editor | Saved filters for the **Voters** tab. They show in the **Custom Filters** menu of that tab. |
| **Enrollment** (under **Voter Authentication**) | **Enabled**, **Disabled** | If voters can enroll. Enrollment applications show on the **Approvals** tab. |
| **OTP** (under **Voter Authentication**) | **Enabled**, **Disabled** | The one-time code (OTP) policy of the sign-in of the voters. |

:::caution CAUTION
Do not set **Locked Down** during the preparation of the election event. It hides the
**Data** tab and the **Scheduled Events** tab, so you cannot undo it from the admin portal.
Contact Sequent support to undo it.
:::

## Buttons

| Button | Use |
| --- | --- |
| **Export** | Opens the **Export Election Event** window. See [Export the election event](#export-the-election-event). |
| **Import Candidates** | Opens a panel to import candidates from a file. In version 9.0 the panel title shows a text code, not a translated title. |
| **Save** | Saves the changes. |

## Export the election event

1. Click **Export**. The **Export Election Event** window opens.
2. Select the content to include. The table gives the options.
3. Click **Export**.
4. If the export is encrypted, the **Password** window shows "Password to decrypt the file:"
   and the password. Copy the password and keep it in a safe location.
5. When the task "Export Election Event" is complete, click **Download File**.

| Option | Content |
| --- | --- |
| **Encrypt with Password** | Encrypts the export file. The admin portal makes the password. |
| **Include Voters** | The voters. |
| **Activity Logs** | The logs of the election event. |
| **Bulletin Board** | The bulletin board. This option turns on **Encrypt with Password**. |
| **Publications** | The publications of the ballot. |
| **S3 Files** | The stored files, for example images and documents. |
| **Scheduled Events** | The scheduled events. |
| **Reports** | The report configurations. This option turns on **Encrypt with Password**. |
| **Applications** | The enrollment applications. This option turns on **Encrypt with Password**. |
| **Tally** | The tally data. This option also selects **Bulletin Board**. |

**Expected result:** you have the export file. To import it later, see
[Create the Election Event](../../../../../procedures/02-event.md#import-an-election-event-alternative).

## If there is a problem

| Problem | Action |
| --- | --- |
| The **Data** tab is not there. | Your role does not have the **View Election Event Data** permission, or the election event is locked down. |
| The **Save** button is not there. | Your role does not have the **Edit Election Event** permission. |
| A language is not in **Language**. | Turn on the language in **Settings** > **LANGUAGES**. |
| "Error exporting Election Event" | The export did not start. Try again. If the error stays, contact Sequent support. |

## Related pages

- [Create the Election Event](../../../../../procedures/02-event.md)
- [Localization](../Localization/election_management_election-event_localization.md)
- [Election: Data](../../Election/Data/election_management_election_data.md)
- [Tasks](../Tasks/election_management_election-event_tasks.md)
