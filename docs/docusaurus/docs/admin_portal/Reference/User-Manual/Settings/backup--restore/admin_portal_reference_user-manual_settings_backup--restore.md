---
id: admin_portal_reference_user_manual_settings_backup___restore
title: Backup / Restore
description: "The Backup / Restore tab saves the configuration of the tenant to a file, and loads a configuration from a file."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Backup / Restore** tab saves the configuration of the tenant to a file, and loads a
configuration from a file. The backup contains the tenant configuration, the sign-in
configuration, and the roles and permissions. It does not contain the election events. To keep
an election event, export it from its **Data** tab.

## Open the tab

1. In the menu on the left, click **Settings**.
2. Click the **Backup / Restore** tab. The title is **Backup / Restore Tenant config**.

## Make a backup

1. Under "Backup Tenant configurations", click **Backup**.
2. Wait for the task "Export Tenant Config" to finish.
3. Keep the downloaded file, `tenant-config-<tenant>-export.zip`, in a safe location.

**Expected result:** you have the backup file.

## Restore a backup

:::caution CAUTION
A restore replaces the tenant configuration, the sign-in configuration and the roles. Do a
restore only with the help of Sequent support.
:::

1. Under "Restore Tenant config", select **Import Tenant Configurations**.
2. Select **Import Keycloak Configurations**.
3. Select **Import Roles & Permissions Configurations**.
4. Click **Restore**. **Restore** is available only when all three options are selected.
5. In the **Import Tenant Configurations** panel, type the SHA-256 hash of the file in
   **Integrity Check (SHA-256)**.
6. Upload the zip file and click **Import**.

**Expected result:** the task "Import Tenant Config" runs. When it is complete, the tenant has
the configuration of the file.

## If there is a problem

| Problem | Action |
| --- | --- |
| **Restore** is not available. | Select all three import options. |
| The task "Export Tenant Config" or "Import Tenant Config" shows `FAILED`. | Read the logs of the task. Contact Sequent support. |

## Related pages

- [Set Up the Tenant](../../../../procedures/01-tenant.md#4-make-a-backup-of-the-tenant-configuration)
- [Settings](../admin_portal_reference_user-manual_settings.md)
