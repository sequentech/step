---
title: Add the Voters
sidebar_position: 3
description: "This procedure makes the voter list of the election event: you prepare the voter file, import it, add or change single voters and check the list."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Add the Voters

This procedure makes the voter list of the election event. You prepare the voter file, import
it, add or change single voters and check the list. The election administrator does this
procedure. You can add voters until the voting period ends.

## Before you start

- The election event and its areas exist. See [Create the Election Event](02-event.md).
- You have the voter list from its official source, as a CSV file or as the data of each voter.
- You know which area each voter belongs to.
- You know how voters sign in: with a password, a one-time code, or another login method. See
  [Tutorials](../01-tutorials/101-admin_portal_tutorials_multi-attribute-password-login.md) for
  the login methods.

:::caution CAUTION
The voter file contains personal data. Keep it on a protected computer. Delete the working
copies after the import, as your data protection rules require.
:::

## 1. Prepare the voter file

1. Open the election event.
2. Click the **Voters** tab.
3. Click **Import**. The **Import Voters** panel opens.
4. Click the link in "Download an example import CSV file here." to download the example file.
5. Make your CSV file from the example. The first row must be the header. The table gives the
   columns.

| Column | Content |
| --- | --- |
| `username` | The user name of the voter. It must be unique in the election event. |
| `email` | The email address. |
| `first_name`, `last_name` | The name of the voter. |
| `area_name` | The name of the area of the voter. It must be the same as on the **Areas** tab. |
| `enabled` | `TRUE` or `FALSE`. A voter with `FALSE` cannot sign in. |
| `password` | Optional. The password of the voter. |
| `sequent.read-only.mobile-number` | Optional. The mobile number, for text messages. |
| `vote-weight` | Optional. A positive whole number. Use it only if the election event uses weighted voting. |

Other columns become attributes of the voter, for example a date of birth for the login. Use the
attribute name as the header, not the label that the admin portal shows. Use a comma as the
separator. For a tab separator, give the file the extension `.tsv`.

**Expected result:** you have a CSV file with one row for each voter and the correct header.

## 2. Import the voters

1. On the **Voters** tab, click **Import**.
2. Upload the CSV file.
3. Click **Import**.

**Expected result:** the message "Voters Import Scheduled Successfully" shows.

4. Click the **Tasks** tab. Wait until the import task is complete.
5. If the task shows an error, open the task and read the error. Correct the file and import
   it again.

**Expected result:** the voters show on the **Voters** tab.

:::note
If some columns are secret voter fields, your account needs the permission to write them. See
[Import Voter List](../01-tutorials/08-admin_portal_tutorials_import-voters.md).
:::

## 3. Add or change one voter

To add a voter:

1. On the **Voters** tab, click **Add**. The **Create Voter** panel opens.
2. Type the fields of the voter.
3. Select the **Area** of the voter.
4. Make sure that **Enabled** is selected.
5. If the voter signs in with a password, type it in **Password** and in **Repeat Password**.
6. Click **Save**.

**Expected result:** the message "Voter created" shows. The voter shows in the list.

To change a voter, click **Edit** in the row of the voter. Change the fields, click **Save**,
read the changes and click **Confirm changes**.

:::note
To change a voter who has already voted, your account needs the **Edit voters who voted**
permission. Without it, the admin portal does not let you change that voter.
:::

## 4. Check the voter list

1. Click the **Dashboard** tab. Compare the number of eligible voters with the number of voters
   in your source.
2. Click the **Voters** tab. Filter the list by **Area**. Compare the voters of each area with
   your source.
3. Open some voters at random. Make sure that their data is correct.

**Expected result:** the voter list matches your source.

## 5. Send the voters their access data (if necessary)

If the voters need a message with their access data before the voting period:

1. On the **Voters** tab, click **Send**. To send to some voters only, select them in the list
   first.
2. Select the recipients: **Everyone**, **Those who didn't vote yet**, **Those who already
   voted**, or the selected voters.
3. Select the communication method and the template.
4. Send the message.

For all the options, see [Voter Communication](../01-tutorials/15-admin_portal_tutorials_voter-communication.md).

To give one voter a printed letter with new access data, click **Voter Information Letter** in
the row of the voter. The admin portal makes an encrypted PDF with a new password.

## If there is a problem

| Problem | Action |
| --- | --- |
| The import task fails. | Open the task on the **Tasks** tab to see the error. Check the header row, the area names and the separator, then import again. |
| A voter has no area. | The `area_name` value does not match an area. Correct the area name and import again, or click **Edit** and select the **Area**. |
| The import rejects a secret column. | Your account needs the permission to write secret voter fields. Ask your tenant administrator. |
| The number of voters is not what you expect. | Look for duplicate user names and empty rows in the file. Compare the voters of each area with your source. |
| "Voter created, but their password could not be set" | The password does not meet the password policy of the election event. Click **Change password** in the row of the voter and type a valid password. |

## More information

- [Create Voters](../01-tutorials/07-admin_portal_tutorials_create-voters.md)
- [Import Voter List](../01-tutorials/08-admin_portal_tutorials_import-voters.md)
- [Configure Password Policy](../01-tutorials/05-admin_portal_tutorials_password-policy.md)
- [Voter Communication](../01-tutorials/15-admin_portal_tutorials_voter-communication.md)
- [Election event: Voters](../02-reference/02-election-event/05-election_management_election-event_voters.md)
- [Election: Voters](../02-reference/03-election/04-election_management_election_voters.md)
- [Election event: Approvals](../02-reference/02-election-event/14-election_management_election-event_approvals.md)

**Next:** [Run the Key Ceremony](04-keys.md).
