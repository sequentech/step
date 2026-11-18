---
id: import_format_tenant_backup
title: Tenant Backup File
sidebar_position: 7
description: "The format of the tenant backup file: the three files in the ZIP, every column and key, and what the restore uses."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The tenant backup file is a ZIP file with the configuration of a tenant. You make it with
**Backup** in **Settings** > **Backup / Restore**. You use it with **Restore** on the same page.
Do not make the file by hand.

Use the backup to keep a copy of the settings of the tenant, or to copy the settings to another
tenant. The backup does not contain election events, voters or votes.

:::caution CAUTION: tenant backup file
Keep the backup file in a protected location. The settings of the tenant can contain the key of an
external service (`gapi_key`). Anyone who has the file can read it.
:::

You need the permission **Read Tenant** (`tenant-read`) to make the backup. You need the
permission **Edit Tenant** (`tenant-write`) to restore it.

## Files in the ZIP

The admin portal downloads the file with the name `tenant-config-<tenant ID>-export.zip`. The ZIP
has three files. `<id>` is the ID of the tenant.

| File | Content | Used by the restore |
| --- | --- | --- |
| `tenant_configuration-<id>.csv` | The settings of the tenant. One header row and one data row. | Option **Import Tenant Configurations** |
| `keycloak_admin-<id>.json` | The login configuration of the tenant (a Keycloak realm), in JSON. | Option **Import Keycloak Configurations**. Only the display name and the texts. |
| `export_permissions-<id>.csv` | The roles and their permissions. | Option **Import Roles & Permissions Configurations** |

The restore finds a file by a part of its name: `tenant_configuration`, `keycloak_admin` and
`export_permissions`. The restore skips the entries that start with `.` or `__MACOSX/` and the
folders. A ZIP file that you made again with a file manager works if the file names are the same.

## Fields of `tenant_configuration-<id>.csv`

The file has a header row and one data row. The restore reads the columns by position, not by name.
The export writes each value as JSON. A text value has its own double quotes, so in the CSV file it shows with three quotes on each side.

| Column | Position | Required | Allowed values or format | Meaning | Used by the restore | Example |
| --- | --- | --- | --- | --- | --- | --- |
| `id` | 0 | Yes | A UUID | The ID of the tenant. | No. The restore changes the tenant that you are signed in to. | `"90505c8a-23a9-4cdf-a26b-4e19f6a097d5"` |
| `slug` | 1 | Yes | Text | The short name of the tenant. | No. The restore reads it but does not save it. | `"example"` |
| `created_at` | 2 | Optional | Date and time in ISO 8601 | When the tenant was made. | Yes | `"2026-01-05T10:00:00+00:00"` |
| `updated_at` | 3 | Optional | Date and time in ISO 8601 | When the tenant changed. | Yes | `"2026-10-01T09:30:00+00:00"` |
| `labels` | 4 | Optional | JSON | Extra data. | Yes | `null` |
| `annotations` | 5 | Optional | JSON object | The look and feel of the tenant: `logo_url` and `css`. | Yes | `{"logo_url":"https://example.com/logo.png","css":""}` |
| `is_active` | 6 | Yes | `true` or `false` | Shows if the tenant is active. | Yes | `true` |
| `voting_channels` | 7 | Optional | JSON object, see below | The voting channels of the tenant. | Yes | `{"online":true,"kiosk":false,"telephone":false}` |
| `settings` | 8 | Optional | JSON object, see below | The settings of the tenant. | Yes | See below. |
| `test` | 9 | Optional | A whole number or `null` | A flag for test tenants. | No. See the note below. | `null` |

If `is_active` is missing or is not `true` or `false`, the restore stops with the error
`Error deserializing is_active`. The restore sets `created_at` and `updated_at` to the values of the
file. If a date cannot be read, the restore saves an empty date.

### Keys of `voting_channels`

| Key | Type | Meaning |
| --- | --- | --- |
| `online` | `true` or `false` | Voting on the internet. If the key is missing, the admin portal shows `true`. |
| `kiosk` | `true` or `false` | Voting in a kiosk. If the key is missing, the admin portal shows `false`. |
| `telephone` | `true` or `false` | Voting by telephone. If the key is missing, the admin portal shows `false`. |

### Keys of `settings`

All keys are optional.

| Key | Type | Meaning |
| --- | --- | --- |
| `language_conf` | Object | The languages. It has `enabled_language_codes` (a list of codes, for example `["en","es"]`), `default_language_code` (for example `"en"`) and `language_detection_policy` (`browser-detect` or `force-default`). |
| `voting_countries` | List of text | The countries where voting is blocked. See **Country Blocking**. |
| `enroll_countries` | List of text | The countries where enrollment is blocked. See **Country Blocking**. |
| `help_links` | List of objects | The help links of the voting portal. Each object has `url`, `title` and optional `i18n`. |
| `i18n` | Object | Texts of the tenant for each language. |
| `sms`, `mail` | `true` or `false` | The channels for messages. |
| `schedules`, `schedulesIds` | List | Scheduled events of the tenant. |
| `gapi_key`, `gapi_email` | Object, text | The credentials of an external service (Integrations). |


### Note on the column `test`

The restore reads the value of `test` from position 8, which is the column `settings`, and not
from position 9. The column `test` of the file is not used. Because `settings` is not a whole
number, the restore saves an empty value (`NULL`) in the field `test` of the tenant. The export
file is not wrong. The restore always clears the field `test`.


## Fields of `keycloak_admin-<id>.json`

The file is the full JSON of the Keycloak realm of the tenant. The restore reads only two parts of it:

| Key | Type | Meaning |
| --- | --- | --- |
| `displayName` | Text | The display name of the realm. The restore sets it in the realm of the tenant. |
| `localizationTexts` | Object | The texts of the login pages for each language. The restore replaces the texts of the realm. |

The restore ignores all other parts of the JSON: users, clients, flows, groups, roles and the
settings of the realm. If a part is missing in the file, the restore does not change it.

## Fields of `export_permissions-<id>.csv`

A CSV file with a header row and two columns. The separator is a comma.

| Column | Position | Required | Format | Meaning | Example |
| --- | --- | --- | --- | --- | --- |
| `role` | 0 | Yes | Text | The name of a group of the realm. In the admin portal this is a **role**. | `Election Manager` |
| `permissions` | 1 | Yes | Names of permissions, separated with `\|` | The permissions of the role. | `voter-create\|voter-read\|tenant-read` |

Example:

```csv
role,permissions
Election Manager,voter-create|voter-read|tenant-read
Auditor,voter-read|tenant-read
```

The export lists the groups of the realm that have realm roles. The order of the rows is not
fixed.

For each row, the restore does this:

1. If the realm has a group with the name in `role`, the restore makes the permissions of the group
   the same as the list. It adds the permissions that are missing. It removes the permissions that
   are not in the list. The restore ignores a permission name that does not exist in the realm.
2. If the realm has no such group, the restore makes the group and adds to it the permissions of all rows read so far, not only the permissions of its own row.

:::caution CAUTION: permissions file of the tenant backup
The restore removes permissions from a role that exists. Check the file before you restore.
:::


## Rules

| Rule | Value |
| --- | --- |
| File type | `.zip` |
| CSV separator | Comma. The encoding is UTF-8. |
| Multiple values | In `permissions`, separate the names with `\|`. |
| File names | Keep the names of the three files. |
| Tenant | The restore changes the tenant that you are signed in to. It does not use the tenant ID in the file. |
| Size | The size is limited by the deployment setting for file uploads. |

## Restore options

The **Restore** panel has three options:

- **Import Tenant Configurations**
- **Import Keycloak Configurations**
- **Import Roles & Permissions Configurations**

:::note NOTE: tenant backup restore options
In this version the **Restore** button works only if you select all three options. The task itself
reads each option on its own. A file that does not have one of the three files gives no error for
that option: the restore does nothing for it.
:::


## Example

A complete ZIP file for a tenant with ID `90505c8a-23a9-4cdf-a26b-4e19f6a097d5`:

```text
tenant-config-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-export.zip
  tenant_configuration-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.csv
  keycloak_admin-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json
  export_permissions-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.csv
```

Content of `tenant_configuration-<id>.csv` (the data row is on one line):

```csv
id,slug,created_at,updated_at,labels,annotations,is_active,voting_channels,settings,test
"""90505c8a-23a9-4cdf-a26b-4e19f6a097d5""","""example""","""2026-01-05T10:00:00+00:00""","""2026-10-01T09:30:00+00:00""",null,"{""logo_url"":""https://example.com/logo.png"",""css"":""""}",true,"{""online"":true,""kiosk"":false,""telephone"":false}","{""language_conf"":{""enabled_language_codes"":[""en"",""es""],""default_language_code"":""en"",""language_detection_policy"":""browser-detect""}}",null
```

## What the restore does

1. The admin portal uploads the file and asks for the **Integrity Check (SHA-256)**. The task
   checks the hash if you typed one.
2. The task **Import Tenant Config** starts. Look at the task widget or the **Tasks** tab.
3. The task reads the realm of the tenant from the login service.
4. For each selected option, the task reads the matching file in the ZIP and changes the tenant
   (database), the realm (display name and texts) or the roles (groups and permissions).
5. The restore changes the existing tenant. It does not make a new tenant.

The task makes the changes in the login service one after the other. If the task fails in the
middle, some changes can remain.

If the database commit fails, the task still shows as completed, and the tenant changes are not saved. Changes made in the login service before the commit remain.

### Messages and errors

| Message | Where | Meaning and action |
| --- | --- | --- |
| Import Tenant Config | Task name | The name of the task in the **Tasks** tab. |
| `Failed to verify the integrity: Hash of voters file: ... does not match with the input hash: ...` | Task log | The hash you typed is not the hash of the file. (The message says "voters file" for this file too.) |
| `Zip archive error: ...` | Task log | The file is not a ZIP file. |
| `Error deserializing is_active` | Task log | The column `is_active` of the tenant file is not `true` or `false`. |
| `Invalid header name: ...` | Task log | A column name of the tenant file has a character that is not allowed. |
| `Failed to deserialize Keycloak realm configuration: ...` | Task log | The JSON file is not valid. |
| `Error obtaining realm` | Task log | The login service did not answer. Try again. |
| `Error adding roles to new group ...`, `Error creating group ...` | Task log | The login service refused a change of the roles. |

## Related pages

- [Set Up the Tenant](../../03-procedures/01-tenant.md): the settings of the tenant.
- [Templates File](06-templates.md): templates are not in the backup. Export them on their own.
- [Import File Formats](index.md): all import files.
