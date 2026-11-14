---
id: election_management_election_event_keys
title: Keys
description: "The Keys tab holds the key ceremonies of the election event. A key ceremony makes the public key that encrypts the ballots, and gives each trustee a fragment of the private key."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Keys** tab holds the key ceremonies of the election event. A key ceremony makes the
public key that encrypts the ballots, and gives each trustee a fragment of the private key.
The election administrator creates the ceremony. Each trustee then saves and checks their
fragment. For the full procedure, see [Run the Key Ceremony](../../../../../procedures/04-keys.md).

## Open the tab

1. In the menu on the left, click the election event.
2. Click the **Keys** tab.

The tab shows with the **View Election Event Keys** permission and one of **Admin Ceremony** or
**Trustee Ceremony**. To create a ceremony, your role needs **Create Ceremony**. The tab is
hidden when the election event is locked down.

## List of ceremonies

When there is no ceremony, the tab shows "No Keys Ceremony yet." and **Create Keys Ceremony**.
When there are ceremonies, the toolbar has **Add**.

| Column | Content |
| --- | --- |
| **Started at** | The date and time of the creation. |
| **Status** | The status of the ceremony. The table below gives the values. |
| **Permission Labels** | The permission labels of the ceremony. See [Permission Labels](../../../../../Tutorials/admin_portal_tutorials_permission-labels.md). |
| **Trustees** | The trustees of the ceremony. |
| **Actions** | The view icon (administrator) or the key icon (trustee). |

| Status | Meaning |
| --- | --- |
| `STARTED` | The ceremony is created. |
| `IN_PROGRESS` | The trustees can download and check their key fragment. |
| `SUCCESS` | All trustees have checked their key fragment. |
| `CANCELLED` | The ceremony is cancelled. |

## Create a ceremony (administrator)

The wizard has three steps: **Configure**, **Ceremony** and **Finished**.

| Field | Use |
| --- | --- |
| **Threshold** | The minimum number of trustees for the tally. The default is 2. It must be 2 or more, and not more than the number of trustees. |
| **Trustees** | The trustees of the ceremony. Use **Filter Trustees** to find a trustee. Select at least as many trustees as the threshold. |
| **Election** | One election, or empty for **All Elections**. Only elections without a key ceremony show. |
| **Create Keys Ceremony** | Opens the question "Are you sure you want to Create Keys Ceremony?". Click **Yes, Create Keys Ceremony**. |

The **Keys Ceremony Progress** page shows **Status: ...**, the trustee table and the **Logs**.
The trustee table has the columns **Trustee Name**, **Key Fragment Generated**, **Private Key
Fragment Downloaded** and **Private Key Fragment Checked**.

:::danger WARNING
Version 9.0 has no button to cancel or restart a key ceremony. Check the threshold, the
trustees and the election before you confirm.
:::

## Take part in a ceremony (trustee)

A trustee sees the message "You have been invited to participate in a Keys ceremony." The
link in the message does not work in version 9.0. Click the key icon in the **Actions** column.

The trustee wizard has four steps: **Start**, **Download**, **Check** and **Finished**.

| Step | What the trustee does |
| --- | --- |
| **Start** | Reads the **Trustee Keys Ceremony** page and clicks **Next**. If "Waiting for Keys Generation.." shows, waits until **Next** is available. |
| **Download** | Clicks **Download your Encrypted Private Key**, copies the file to two storage devices, selects **First backup secured** and **Second backup secured**, and clicks **Confirm Backups and Continue**. |
| **Check** | Uploads one backup on the **Check your Encrypted Private Key Backups** page. Waits for "Backup verified successfully." and clicks **Next**. |
| **Finished** | Sees the progress of the ceremony. |

## If there is a problem

| Problem | Action |
| --- | --- |
| "You selected threshold ... but it must be between ... and ...." | Type a threshold between 2 and the number of trustees. |
| "You selected only ... trustees, but you must select at least ...." | Select more trustees, or type a lower threshold. |
| "Error creating Keys Ceremony: ..." with "there's already an existing running ceremony ..." | The election, or all the elections, already have a key ceremony. Contact Sequent support. |
| "Cannot create keys ceremony: one or more permission labels are missing." | Your account does not have the permission label of the election. See [Permission Labels](../../../../../Tutorials/admin_portal_tutorials_permission-labels.md). |
| "Invalid Encrypted Private Key Backup, please try again" | Upload the file from the other storage device. |
| "Private key download is no longer available because the ceremony has moved on." | Use your backups. You cannot download the file again. |
| "Your private key was already downloaded and verified." | You have done your part. No action is necessary. |
| The check step shows `keysGeneration.checkStep.noFileSelected`. | No file was selected. Upload the file again. This text is not translated in version 9.0. |

## Related pages

- [Run the Key Ceremony](../../../../../procedures/04-keys.md)
- [Settings: Trustees](../../../Settings/Trustees/admin_portal_reference_user-manual_settings_trustees.md)
- [Tally](../Tally/election_management_election-event_tally.md)
