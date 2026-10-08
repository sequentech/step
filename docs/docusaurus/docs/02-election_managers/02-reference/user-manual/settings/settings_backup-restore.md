---
id: settings_backup-restore
title: Backup & Restore
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Backup / Restore** tab is in **Settings**. Its title is **Backup / Restore Tenant
config**. Use it to make a backup file of the tenant configuration, and to restore a
configuration from such a file. The tenant administrator uses it.

The backup does not contain election events, voters, users or votes. It contains only the
configuration of the tenant.

## How to open it

1. In the menu on the left, click **Settings**.
2. Click the **Backup / Restore** tab.

## Required permission

- To open **Settings**, your account must have the **Edit Tenant** and **View settings**
  permissions.
- To make a backup, your account must have the **Read Tenant** permission.
- To restore, your account must have the **Edit Tenant** permission.

## Content of the backup file

The backup is a ZIP file with the name `tenant-config-<tenant ID>-export.zip`. It contains
three files:

| File content | What it contains | Restore option |
| --- | --- | --- |
| Tenant configuration | The settings of the tenant, for example the languages, the voting channels, the look and feel, the country lists and the help links. | **Import Tenant Configurations** |
| Sign-in configuration | The configuration of the sign-in service (Keycloak) of the tenant. | **Import Keycloak Configurations** |
| Roles and permissions | Each role of the tenant and its permissions. | **Import Roles & Permissions Configurations** |

## Fields and buttons

| Item | Meaning |
| --- | --- |
| **Backup Tenant configurations** / **Backup** | Starts a task that makes the backup file. The browser then downloads the file. |
| **Restore Tenant config** | The section to restore a backup file. |
| **Import Tenant Configurations** | A checkbox. Restores the tenant configuration. |
| **Import Keycloak Configurations** | A checkbox. Restores the display name and the texts of the sign-in pages. Other sign-in settings do not change. |
| **Import Roles & Permissions Configurations** | A checkbox. Restores the roles and their permissions. |
| **Restore** | Opens the panel **Import Tenant Configurations**. The button works only when all three checkboxes are selected. |
| **Integrity Check (SHA-256)** | In the restore panel. The SHA-256 hash of the backup file. The system compares the file with this value. |
| **Cancel** / **Import** | In the restore panel. **Import** starts the restore. It is available after the file upload is complete. |

## Make a backup

1. Open the **Backup / Restore** tab.
2. Click **Backup**.
3. Wait for the task **Export Tenant Config** to finish.
4. Wait for the browser to download the file `tenant-config-<tenant ID>-export.zip`.
5. Keep the file in a safe location.

**Expected result:** you have the backup file.

Make a backup after you set up the tenant and after each important change.

## Restore a backup

:::caution CAUTION
Do a restore only with the help of Sequent support. A restore overwrites the tenant
configuration. For each role in the file, the restore sets the permissions of the role to the
permissions in the file. You cannot undo a restore, except with a newer backup.
:::

1. Make a backup of the current configuration. Use the steps in
   [Make a backup](#make-a-backup).
2. Select **Import Tenant Configurations**.
3. Select **Import Keycloak Configurations**.
4. Select **Import Roles & Permissions Configurations**.
5. Click **Restore**. The panel **Import Tenant Configurations** opens.
6. In **Integrity Check (SHA-256)**, type the SHA-256 hash of the backup file.
7. Drag the backup ZIP file to the panel, or click the panel to select the file.
8. Wait for the message "File uploaded to server - but not imported yet".
9. Click **Import**.
10. Wait for the task **Import Tenant Config** to finish.

**Expected result:** the task shows that it is complete. The settings and the roles show the
values of the backup.

:::note
If you leave **Integrity Check (SHA-256)** empty, the admin portal shows "Import Without
Integrity Check?". Click **Yes, Import without Integrity Check** only if you are sure that the
file is correct. Click **Go Back** to type the hash.
:::

A role that is in the file but not in the tenant is created. A role that is in the tenant but
not in the file does not change.

## If there is a problem

| Problem | Action |
| --- | --- |
| The **Restore** button does not work. | Select all three checkboxes. The button works only when all three are selected. |
| The message "Error uploading file" shows. | Select the file again. Make sure that it is the ZIP file of a backup. |
| The task **Export Tenant Config** or **Import Tenant Config** fails. | Make sure that your account has the necessary permission. Then try again. If the task fails again, contact Sequent support. |
| The restore task fails with a message about the integrity or the hash. | The hash that you typed is not the hash of the file. Check the hash and the file, then try again. |
| The browser does not download the backup file. | Make sure that the browser permits downloads from the admin portal. Click **Backup** again. |

**Related procedure:** [Set Up the Tenant](../../../03-procedures/01-tenant.md),
step 4.
