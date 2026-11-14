---
id: election_management_election_event_transmission
title: Transmission
description: "A transmission package holds the results of one election in one area. You make it from a completed tally, the signers sign it, and you send it to the destination servers."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

A transmission package holds the results of one election in one area. You make the package
from a completed tally. The signers of the area can sign the package with their signing file.
Then you send the package to the destination servers, or you download it.

The transmission feature is available only when Sequent has turned it on for your platform.
Sequent also configures the destination servers and the signers of each area.

## Before you start

- The tally has the type **Electoral Results** and the status `SUCCESS`. See
  [Run the Tally Ceremony](../../../../../procedures/06-tally.md).
- The destination servers and the signers of each area are configured.
- Your role has the permissions for the actions that you do:

| Permission | Action |
| --- | --- |
| **Admin Ceremony**, **Miru Create**, **Miru Sign**, **Miru Send** or **Miru Download** | See the view icon of the tally. |
| **Export Ceremony** | Use the **Actions** menu of an election, to open or make a package. |
| **Miru Create** | Make a package, and use **Regenerate**. |
| **Miru Sign** | Sign a package. |
| **Miru Send** | Use **Send**. |
| **Miru Download** | Use **Download**. |
| **Generate Transmission Report** | Use **Download Transmission Report**. |

## Open a transmission package

1. Open the election event.
2. Click the **Tally** tab.
3. In the row of the tally, click the view icon. Its tooltip is **View Tally Ceremony**. If your
   role has **Trustee Ceremony**, the icon is an antenna and its tooltip is **Manage Tally
   Ceremony**.
4. Open **Results & Participation**.
5. In **Elections**, click the tab of the election.
6. Click **Actions**, at the right of the tabs.
7. Click **Generate Transmission Package for Area '&lt;area&gt;'**. The menu has one item for each
   area that has results in the election.

**Expected result:** if the package exists, the package page opens. If not, the platform makes
the package with the task **Create Transmission Package**. The package page opens when the
package is ready.

## Package page

The title is "Transmission Package for Area '&lt;area&gt;' and Election '&lt;election&gt;'". The page has
these buttons. A button shows only when your role has its permission.

| Button | Use |
| --- | --- |
| **Send** | Sends the package to the destination servers. |
| **Download** | Downloads the package files or the transmission report. |
| **Regenerate** | Makes a new version of the package. |
| **Back** | Goes back to the tally. |

The page has these sections:

| Section | Content |
| --- | --- |
| **Sign Transmission Package** | The upload area for your signing file. It shows only when your user is a signer of the area. |
| **SBEI Signatures** | The signers of the area, with the columns **SBEI id** and **Has Signed**. A check mark shows a signature. An hourglass shows that the signer has not signed. |
| **Destination Servers** | The destination servers, with the columns **Server Name** and **Sending Status**. A check mark shows that the server received the package. An hourglass shows that it did not. |
| **Logs** | The events of the package, with **Date** and **Entry**. |

The title of **SBEI Signatures** shows the status "&lt;n&gt; out of &lt;total&gt; Signed, &lt;minimum&gt; minimum".
The title of **Destination Servers** shows the status "Sent to &lt;n&gt; out of &lt;total&gt;".

## Sign the package (signer)

Do this procedure only if the **Sign Transmission Package** section shows. The section says that
signing is optional. But **Send** needs the minimum number of signatures that the status shows.

Have your signing file and its password.

:::note
Use the same signing file each time. After your first signature, the platform accepts only that
file for your user. The platform refuses a file that signed a package of a different election.
:::

1. Open the package. See [Open a transmission package](#open-a-transmission-package).
2. In **Sign Transmission Package**, drag your signing file to the upload area. Or click
   **Browse** and select the file.
3. In "Do you want to sign the Transmission Package?", type the password of the file in
   **Enter your password**.
4. Click **Sign Transmission Package**.

**Expected result:** the message "Signing Successful" shows. In **SBEI Signatures**, your row
shows a check mark.

## Send the package

:::caution CAUTION
Check the results before you send the package. You cannot cancel a transmission.
:::

1. Make sure that the number of signatures is equal to or more than the minimum.
2. Click **Send**.
3. Read "Do you want to Send the Transmission Package?".
4. Click **Send Transmission Package**.

**Expected result:** the message "Sending Transmission Package..." shows. In **Destination
Servers**, each server that received the package shows a check mark.

**Send** is not available when the package has fewer signatures than the minimum, or when all
destination servers received the package. The tooltip then shows "Have not reached minimum
number of SBEI Member signatures or Transmission Package has already been sent to all servers".

## Download the package or the report

1. Click **Download**.
2. Click an item of the menu:

| Item | Content |
| --- | --- |
| **Download EML** | The results file of the package. The item shows only when the package has this file. |
| **Download Transmission Package &lt;date&gt;** | A ZIP file with the package for all destination servers. |
| **Download Transmission Report** | A PDF report of the transmission. The platform makes it with the task **Generate Transmission Report**. |

The date is the date and time of the newest version of the package.

3. For the first two items, read "Do you want to Download the Transmission Package?".
4. Click **Download Transmission Package**.

**Expected result:** the browser saves the file.

## Regenerate the package

:::caution CAUTION
Regenerate the package only if it is necessary. The new version has no signatures and is not
sent. The signers must sign it again, and you must send it again.
:::

1. Click **Regenerate**.
2. Read "Do you want to regenerate the Transmission Package?".
3. Click **Regenerate Transmission Package**.

**Expected result:** the platform makes a new version of the package. **SBEI Signatures** and
**Destination Servers** show the status of the new version.

## If there is a problem

| Problem | Action |
| --- | --- |
| The **Actions** menu has no **Generate Transmission Package for Area** item. | Make sure that the tally has the type **Electoral Results** and that the area has results. If the item is still not there, contact Sequent support. |
| "Error creating Transmission Package" | Try again. If the error continues, contact Sequent support. |
| "Error sending Transmission Package" | Read the **Logs** section. Then contact Sequent support. |
| The **Sign Transmission Package** section does not show. | Your user is not a signer of the area. Contact Sequent support. |
| "Invalid Encrypted Private Key Backup, please try again" after you add the signing file. | The file did not upload. Add the file again. |
| "There was an error uploading signature" | Make sure that the file and the password are correct. Use the file of your first signature. |
| **Send** is not available. | Wait until enough signers have signed. If all servers show a check mark, the package is already sent. |
| A server shows an hourglass after you send. | The platform does not send again to a server after a failed attempt. Read the **Logs** section. Then contact Sequent support. |

## Related pages

- [Run the Tally Ceremony](../../../../../procedures/06-tally.md)
- [Get the Results](../../../../../procedures/07-results.md)
- [Election event: Tally](election_management_election-event_tally.md)
- [Election event: Tasks](../Tasks/election_management_election-event_tasks.md)
- [Roles](../../../Users-and-Roles/Roles/admin_portal_reference_user-manual_users-and-roles_roles.md)
