---
title: Add the Voters
sidebar_position: 3
description: "This procedure makes the voter list of the election event. You prepare the voter file, import it, add or change single voters and check the list. The election administrator does this procedure."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Add the Voters

This procedure makes the voter list of the election event. You prepare the voter file, import
it, add or change single voters and check the list. The election administrator does this
procedure.

## Before you start

- The election event, its contests and its areas exist. See
  [Create the Election Event](02-event.md).
- You have the voter list from its official source, as a CSV file or as the data of each voter.
- You know the area of each voter. The area names must be the same as on the **Areas** tab.
- Your account has the permissions to create and import voters. If the **Add** or **Import**
  button is not on the **Voters** tab, ask your tenant administrator.

:::caution CAUTION
The voter file contains personal data. Keep it on a protected computer. Delete the working
copies after the import, as your data protection rules require.
:::

## 1. Prepare the voter file

Version 9.0 does not supply an example file in the admin portal. Make the file from the table
below.

1. Make a CSV file with one row for each voter.
2. Put the header in the first row. Use the column names of the table.
3. Use a comma as the separator. For a tab separator, give the file the extension `.tsv`.
4. Save the file.

| Column | Content |
| --- | --- |
| `username` | The user name of the voter. Use a different value for each voter. |
| `email` | Optional. The email address. |
| `first_name`, `last_name` | Optional. The name of the voter. |
| `area_name` | The name of the area of the voter. It must be the same as on the **Areas** tab. |
| `enabled` | `TRUE` or `FALSE`. A voter with `FALSE` cannot sign in. |
| `password` | Optional. The password of the voter. |
| `sequent.read-only.mobile-number` | Optional. The mobile number, for text messages. |

A column with another name becomes an attribute of the voter. A column name can contain only
letters, digits, `.`, `_` and `-`.

**Expected result:** you have a CSV file with one row for each voter and the correct header.

## 2. Import the voters

1. Open the election event.
2. Click the **Voters** tab.
3. Click **Import**. The **Import Voters** panel opens.
4. Type the SHA-256 hash of the file in **Integrity Check (SHA-256)**.
5. Drop the file in the upload area, or click **Browse** and select the file.
6. Wait for the message "File uploaded to server - but not imported yet".
7. Click **Import**.

**Expected result:** the message "Voters Import Scheduled Successfully" shows.

:::note
The upload area shows "Supported format: txt". This text is not correct for the voter import.
Upload the CSV or TSV file.
:::

8. Click the **Tasks** tab. Find the task **Import Users**.
9. Wait until its status is `SUCCESS`.
10. If the status is `FAILED`, click the view icon of the task and read the logs. Correct the
    file and import it again.

**Expected result:** the voters show on the **Voters** tab.

:::caution CAUTION
If you do not type the hash, the admin portal asks "Import Without Integrity Check?". Click
**Go Back** and type the hash. Without the hash, you cannot know if somebody changed the file.
:::

## 3. Add or change one voter

To add a voter:

1. On the **Voters** tab, click **Add**. If the list is empty, click **Create Voter**.
2. Type the fields of the voter.
3. Make sure that **Enabled** is selected.
4. In **Area**, type three or more letters of the area name, then select the area.
5. If the voter signs in with a password, type it in **Password:** and in **Repeat Password:**.
6. If the voter must change the password at the first sign-in, keep **Temporary** on.
7. Click **Save**.

**Expected result:** the message "Voter created" shows. The voter shows in the list.

To change a voter:

1. In the row of the voter, open the actions menu and click **Edit**.
2. Change the fields.
3. Click **Save**.

**Expected result:** the message "Voter edited" shows.

To give a voter a new password, click **Change password** in the actions menu of the voter.

## 4. Check the voter list

1. Click the **Dashboard** tab. Compare **Eligible Voters** with the number of voters in your
   source.
2. Click the **Voters** tab. Click **Add filter**, then add the **Area** filter.
3. Compare the voters of each area with your source.
4. Look for voters with `-` in the **Area** column. These voters have no area.
5. Open some voters at random. Make sure that their data is correct.

**Expected result:** the voter list matches your source.

## 5. Send the voters a message (if necessary)

Do this step only if the voters must get a message before the voting period, for example their
access data. Your tenant needs a template of the correct type. See
[Templates](../Reference/User-Manual/Templates/admin_portal_reference_user-manual_templates.md).

1. On the **Voters** tab, click **Send**. To send to some voters only, select them in the list,
   then click **Send** above the list.
2. In **Audience**, select **Everyone**, **Those who didn't vote yet**, **Those who already
   voted** or the selected voters.
3. In **Schedule**, keep **Send now** on, or type the date and time to start.
4. In **Communication Template**, select the **Template Method**, the **Communication Type** and
   the **Template Alias**.
5. Check the text of the message.
6. Click **Send Notification**.

**Expected result:** the message "Notification programmed/sent successfully" shows.

## If there is a problem

| Problem | Action |
| --- | --- |
| The **Import Users** task shows `FAILED`. | Open the task on the **Tasks** tab and read the logs. Check the header row, the column names and the separator, then import again. |
| A voter shows `-` in the **Area** column. | The `area_name` value did not match an area. In the actions menu of the voter, click **Edit** and select the **Area**. |
| "Import Without Integrity Check?" | Click **Go Back** and type the SHA-256 hash of the file. |
| "Error importing Voters" | The import did not start. Upload the file again. If the error stays, contact Sequent support. |
| "Passwords must match" | Type the same password in **Password:** and in **Repeat Password:**. |
| "Error creating voter" | Check the required fields, for example the user name, and the area. A user name can be used only once. |
| The number of voters is not what you expect. | Look for duplicate user names and empty rows in the file. Compare the voters of each area with your source. |

## More information

- [Election event: Voters](../Reference/User-Manual/Election-Management/Election-Event/Voters/election_management_election-event_voters.md)
- [Election: Voters](../Reference/User-Manual/Election-Management/Election/Voters/election_management_election_voters.md)
- [Election event: Areas](../Reference/User-Manual/Election-Management/Election-Event/Areas/election_management_election-event_areas.md)
- [Election event: Approvals](../Reference/User-Manual/Election-Management/Election-Event/Approvals/election_management_election-event_approvals.md)
- [Election event: Tasks](../Reference/User-Manual/Election-Management/Election-Event/Tasks/election_management_election-event_tasks.md)
- [Templates](../Reference/User-Manual/Templates/admin_portal_reference_user-manual_templates.md)

**Next:** [Run the Key Ceremony](04-keys.md).
