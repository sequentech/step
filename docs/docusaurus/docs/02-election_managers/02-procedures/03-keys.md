---
title: "03-KEYS: Run the key ceremony"
sidebar_label: "03-KEYS: Run the key ceremony"
sidebar_position: 3
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# 03-KEYS: Run the key ceremony

The key ceremony makes the keys of the election. The voting portal encrypts each ballot with
the public key. The private key is in fragments: each trustee keeps one fragment. To decrypt
the votes, a minimum number of trustees (the threshold) must give their fragment at the tally
ceremony.

The election administrator starts the ceremony. Then each trustee downloads, saves and checks
their key fragment.

## Before you start

- All the elections of the election event exist. See [02-EVENT](02-event.md).
- Each trustee has an account with the `trustee` role and the correct **Act as Trustee**
  value. See [01-TENANT](01-tenant.md).
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
3. Click **Create Keys Ceremony**. If the election event already has a key ceremony, click
   **Add**.
4. In **Threshold**, type the threshold.
5. In **Trustees**, select the trustees of the ceremony. Select a number of trustees that is
   equal to or more than the threshold.
6. If the election event permits automatic ceremonies, decide about **Automatic Ceremony**.
   Leave it off for an election with trustees. See the warning in
   [02-EVENT](02-event.md#2-configure-the-election-event).
7. In **Election**, select one election. To make one key for all the elections, leave the field
   empty. The ceremony then has the name **All Elections**.
8. Click **Create Keys Ceremony**.
9. Read the message "Are you sure you want to Create Keys Ceremony?".
10. Click **Yes, Create Keys Ceremony**.

**Expected result:** the message "Keys Ceremony created" shows. The ceremony shows in the list
of the **Keys** tab. Its status is `STARTED`, and then `IN_PROGRESS`.

11. Tell the trustees that the key ceremony has started. For an automatic ceremony, the trustees
    do nothing. Go to [step 3](#3-check-that-the-ceremony-is-complete-election-administrator).

## 2. Save the key fragment (each trustee)

Each trustee does these steps on their own computer, with their own account.

1. Sign in to the admin portal.
2. Open the election event.
3. Click the **Keys** tab. The message "You have been invited to participate in a Keys
   ceremony." shows.
4. In the row of the ceremony, click the key icon **Participate in Keys Ceremony** in the
   **Actions** column.
5. Read the **Trustee Keys Ceremony** page. Make sure that the trustee name on the page is
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
3. Click the view icon of the ceremony. The **Keys Ceremony Progress** page opens.
4. Make sure that each trustee has a tick in these columns:
   - **Key Fragment Generated**
   - **Private Key Fragment Downloaded**
   - **Private Key Fragment Checked**

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

**Next:** [04-PUBLISH: Publish the ballot and manage the voting period](04-publish.md).
