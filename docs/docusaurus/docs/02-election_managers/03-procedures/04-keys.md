---
title: Run the Key Ceremony
sidebar_position: 4
description: "The key ceremony makes the keys of the election. The voting portal encrypts each ballot with the public key. The private key is in fragments: each trustee keeps one fragment."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Run the Key Ceremony

The key ceremony makes the keys of the election. The voting portal encrypts each ballot with
the public key. The private key is in fragments: each trustee keeps one fragment. To decrypt
the votes, a minimum number of trustees (the threshold) must give their fragment at the tally
ceremony.

The election administrator starts the ceremony. Then each trustee downloads, saves and checks
their key fragment.

## Before you start

- All the elections of the election event exist. See [Create the Election Event](02-event.md).
- Each trustee has an account with the `trustee` role and the correct **Act as Trustee**
  value. See [Set Up the Tenant](01-tenant.md).
- You know the threshold. It must be 2 or more, and not more than the number of trustees.
- Each trustee has two storage devices for the backups, for example two USB flash drives.
- You have agreed a date and a time with the trustees. All trustees must do their part.

:::danger WARNING
You cannot cancel or restart a key ceremony from the admin portal. Check the
threshold, the trustees and the election before you confirm. If you must change a key
ceremony, contact Sequent support.
:::

:::caution CAUTION
A key ceremony for **All Elections** stops you from making other key ceremonies in the
election event. Create all the elections before you start it. If you add an election later, it
cannot get keys.
:::

## 1. Create the key ceremony (election administrator)

1. Open the election event.
2. Click the **Keys** tab.
3. Click **Create Key Ceremony**. If the election event already has a key ceremony, click
   **Add**.
4. In **Threshold**, type the threshold.
5. In **Trustees**, select the trustees of the ceremony. Select a number of trustees that is
   equal to or more than the threshold.
6. If the election event permits automatic ceremonies, decide about **Automatic Ceremony**.
   Leave it off for an election with trustees. See the warning in
   [Create the Election Event](02-event.md#2-configure-the-election-event).
7. In **Election**, select one election. To make one key for all the elections, leave the field
   empty. The ceremony then has the name **All Elections**.
8. Click **Create Key Ceremony**.
9. Read the message "Are you sure you want to Create Key Ceremony?".
10. Click **Yes, Create Key Ceremony**.

![The Create Election Event Key Ceremony page](../01-tutorials/assets/keys_config_panel.png)

**Expected result:** the message "Key Ceremony created" shows. The ceremony shows in the list
of the **Keys** tab. Its status is `STARTED`, and then `IN_PROGRESS`.

11. Tell the trustees that the key ceremony has started. For an automatic ceremony, the trustees
    do nothing. Go to [step 3](#3-check-that-the-ceremony-is-complete-election-administrator).

## 2. Save the key fragment (each trustee)

Each trustee does these steps on their own computer, with their own account.

![The invitation message and the key icon in the Actions column](../01-tutorials/assets/keys_trustee_actions.png)

1. Sign in to the admin portal.
2. Open the election event.
3. Click the **Keys** tab. The message "You have been invited to participate in a Keys
   ceremony." shows.
4. In the row of the ceremony, click the key icon **Participate in Keys Ceremony** in the
   **Actions** column.
5. Read the **Trustee Key Ceremony** page. Make sure that the trustee name on the page is
   your trustee.
6. Click **Next**.

If the message "Waiting for Keys Generation.." shows, the trustee services have not made the
keys yet. Wait one or two minutes. **Next** becomes available.

7. Click **Download your Encrypted Private Key**. The browser saves a file with the name
   `encrypted_private_key_trustee_<user>_<election>.txt`.
8. Copy the file to your first storage device.
9. Copy the file to your second storage device.
10. Click **Next**. The **Backup your Encrypted Private Key** window opens.
11. Select **First backup secured**.
12. Select **Second backup secured**.
13. Click **Confirm Backups and Continue**.
14. Drop the file from one storage device into the upload area of the **Check your Encrypted
    Private Key Backups** page.
15. Make sure that the message "Backup verified successfully." shows.
16. Click **Next**.
17. Delete the downloaded file from the computer. Keep only the two copies on the storage
    devices.
18. Keep the two storage devices in two different safe locations.

:::danger WARNING
Do not give your key fragment to another person, and do not send it by email or chat. Keep it
until the end of the tally ceremony and the end of the period for claims. If too many trustees
lose their fragment, nobody can decrypt the votes.
:::

![The Download Encrypted Private Key step](../01-tutorials/assets/keys_download_key.png)

![The Backup your Encrypted Private Key window](../01-tutorials/assets/keys_secure_backups.png)

![The Check your Encrypted Private Key Backups step](../01-tutorials/assets/keys_backup_verification.png)

**Expected result:** the **Finished** step shows the progress of the ceremony.

### Check a backup again (optional)

You can check a backup later, for example before the tally ceremony. Do this while the ceremony
status is `IN_PROGRESS` or `SUCCESS`.

1. On the **Keys** tab, open the ceremony.
2. Click **Verify key**.
3. Drop the file from a storage device into the upload area.
4. Make sure that the message "Backup verified successfully." shows.

## 3. Check that the ceremony is complete (election administrator)

1. Open the election event.
2. Click the **Keys** tab.
3. Click the view icon of the ceremony. The **Key Ceremony Progress** page opens.
4. Make sure that each trustee has a tick in these columns:
   - **Key Fragment Generated**
   - **Private Key Fragment Downloaded**
   - **Private Key Fragment Checked**

![The Key Ceremony Progress page with the trustee table](../01-tutorials/assets/keys_ceremony_status.png)

**Expected result:** the status is `SUCCESS`. The status changes to `SUCCESS` only when all
trustees have checked their key fragment. An automatic ceremony changes to `SUCCESS` when the
public key exists.

4. Record the date, the time, the trustees present and the result of the ceremony.

## Ceremony status

| Status | Meaning |
| --- | --- |
| `STARTED` | The ceremony is created. |
| `IN_PROGRESS` | The trustees can download and check their key fragment. |
| `SUCCESS` | All trustees have checked their key fragment. The ceremony is complete. |
| `CANCELLED` | The ceremony is cancelled. |

## If there is a problem

| Problem | Action |
| --- | --- |
| "You selected threshold ... but it must be between ... and ..." | Type a threshold between 2 and the number of trustees. |
| "You selected only ... trustee(s), but you must select at least ..." | Select more trustees, or type a lower threshold. |
| The **Keys** tab is not there for a trustee. | The account does not have the `trustee` role. Ask your tenant administrator. |
| The trustee does not see the key icon. | Make sure that **Act as Trustee** of the account is the trustee of the ceremony. |
| "Invalid Encrypted Private Key Backup, please try again" | Use the file from the other storage device. If both fail, click **Back** and download the file again. |
| "Private key download is no longer available because the ceremony has moved on." | You cannot download the file again. Use your backups. |
| "there's already an existing running ceremony ..." | The election, or all the elections, already have a key ceremony. Contact Sequent support if you must replace it. |
| The status stays at `IN_PROGRESS`. | A trustee has not checked their key fragment. Look at the trustee table and contact that trustee. |

## More information

- [Key ceremony (with screenshots)](../01-tutorials/10-admin_portal_tutorials_key-ceremony.md)
- [Election event: Keys](../02-reference/02-election-event/07-election_management_election-event_keys.md)
- [Settings: Trustees](../02-reference/user-manual/settings/settings_trustees.md)

**Next:** [Publish and Manage the Voting Period](05-publish.md).
