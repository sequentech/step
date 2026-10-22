---
id: import_format_private_key_backup
title: Private Key Backup File
sidebar_position: 9
description: "The private key backup file: its name and content, how the admin portal checks it, the results of the check and how to keep the file safe."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The private key backup file holds the key fragment of one trustee, in an encrypted form. Each
trustee downloads the file during the keys ceremony. The trustee uploads the same file again to
check it. Later, in the tally ceremony, the trustee uploads the file again to give the key
fragment to the platform. You do not make this file yourself. You only download it, keep it and
upload it without any change.

For the steps, see [Run the Key Ceremony](../../procedures/04-keys.md) and
[Run the Tally Ceremony](../../procedures/06-tally.md).

:::danger WARNING: private key backup file
Keep at least two copies of the file in two different secure places. The tally needs enough
trustees to upload a valid file. The number of trustees is set by the threshold of the keys
ceremony. If too few trustees keep a valid file, the votes cannot be decrypted. Do not share the
file with other people.
:::

## What the file is

| Property | Value |
| --- | --- |
| File name | `encrypted_private_key_trustee_<user name>_<election event name>.txt` |
| `<user name>` | The user name of the trustee who downloads the file. |
| `<election event name>` | The name (alias) of the election event, as the admin portal shows it. |
| Type | Plain text, one line, no header. |
| Content | The encrypted private key fragment of the trustee, written in Base64 (standard alphabet, no `=` padding). |
| One file for | One trustee and one keys ceremony. The file of another trustee does not work. |

Example of the name: `encrypted_private_key_trustee_trustee1_Election Event 1.txt`.

:::note NOTE: private key backup file
The content of the file is not an example here on purpose. Never publish a real file.
:::

## Rules

| Rule | Value |
| --- | --- |
| Do not change the file | The platform compares the full text of the file with the text that it keeps. One different character, a new line at the end or a space gives the result **Invalid**. |
| Do not open it in an editor that saves changes | The editor can add a new line at the end of the file. |
| File name | You can rename the file. The platform does not check the name. |
| Who can upload | Only the trustee that owns the file. The check uses the trustee in your session. |
| Encoding | Plain text. The admin portal reads the file as text. |

## Download in the keys ceremony

A trustee can download the file only when these conditions are true:

- The keys ceremony has the execution status `IN_PROGRESS`.
- The trustee is part of the keys ceremony.
- The trustee has not yet checked the key.

If the trustee already checked the key, the admin portal shows "Your private key was already
downloaded and verified." If the ceremony has moved on, it shows "Private key download is no
longer available because the ceremony has moved on." If the platform has no key, it shows "Download
error, empty file". If another error occurs, it shows "The private key could not be downloaded.
Please try again."

When the trustee clicks **Download your Encrypted Private Key**, the status of the trustee changes
to *Private Key Fragment Downloaded*. The message "Encrypted Private Key downloaded successfully."
shows. The **Next** button stays disabled until the download succeeds. Then the trustee must confirm
two backups: **First backup secured** and **Second backup secured**.

## Check in the keys ceremony

In the step **Check your Encrypted Private Key Backups**, the trustee uploads a backup file. The
trustee can try as many times as needed, with different copies. The check works only while the
keys ceremony has the execution status `IN_PROGRESS` or `SUCCESS`.

| Result | Message | Meaning |
| --- | --- | --- |
| Valid | "Backup verified successfully." | The file is the same as the file that the platform keeps. The status of the trustee changes to *Private Key Fragment Checked*. |
| Not valid | "Invalid Encrypted Private Key Backup, please try again" | The file is different. Upload another copy. |

When all trustees have checked their files, the keys ceremony changes to the status `SUCCESS`.

## Use in the tally ceremony

In the tally ceremony, each trustee gives the key fragment to the platform with the same file. The
page shows the text "Please upload you key fragment". A new key fragment is accepted only when both
conditions are true:

- The status of the trustee in this tally is `WAITING`.
- The tally has the execution status `STARTED` or `CONNECTED`.

The platform compares the file with the key that it keeps. There are three results:

| Result | Message | Meaning |
| --- | --- | --- |
| `Restored` | "Backup verified successfully." | The file is correct. The status of the trustee changes to `KEY_RESTORED`. |
| `AlreadyRestored` | "Your key was already restored." | The file is correct, and this trustee gave the key before. Nothing changes. |
| `Invalid` | "Invalid Encrypted Private Key Backup, please try again" | The file is different from the key that the platform keeps. Upload another copy. |

The platform checks the file first. If the file is different, the result is `Invalid`, even when the
tally does not accept a new key. If the file is correct but the tally or the trustee has another
status, the upload fails with an error. The page then shows the same message as for an invalid file.
Look at the status of the tally for more information.

When the number of trustees with the status `KEY_RESTORED` is the same as the threshold of the keys
ceremony or more, the tally changes to the status `CONNECTED`. The platform writes an entry in the
electoral log for each restored key.

## How to keep the file safe

1. Download the file on a device that only you use.
2. Copy the file to two different secure devices. For example, two encrypted USB drives.
3. Keep the devices in two different places.
4. Check each copy in the check step of the keys ceremony before you continue.
5. Do not send the file by email or by a chat application.
6. Do not store the file in a shared folder.

## Related pages

- [Run the Key Ceremony](../../procedures/04-keys.md): download and check the file.
- [Run the Tally Ceremony](../../procedures/06-tally.md): upload the file again.
- [Keys](../User-Manual/Election-Management/Election-Event/Keys/election_management_election-event_keys.md): the **Keys** tab.
- [Import File Formats](index.md): all import files.
