---
id: settings_logs
title: Logs
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

**Settings** > **Logs** shows the tenant log: the record of what happens in the organization outside
its election events. It looks and works like the
[Logs tab of an election event](../../02-election-event/11-election_management_election-event_logs.md),
which keeps recording what happens inside that event.

The tab's subtitle reads: "Sign-ins, staff accounts and roles, settings and election events of this
organization. Each election event keeps its own log in its Logs tab."

## What the tenant log records

Every entry is signed. An entry about a change made by a person is signed with that person's admin
key and with the log's own system key; the other entries are signed with the system key alone.

| Area | Entries |
|---|---|
| Sign-in and account security | Sign-ins to the Admin Portal, failed sign-ins, sign-outs, temporary and permanent lockouts, password changes and resets, authenticator set-up and removal, impersonation, and every Keycloak event that ended in an error. Successful token refreshes and code exchanges are not logged. |
| Staff accounts | Created, edited, enabled, disabled, password reset by an administrator, deleted, a role given or removed, imported and exported. An import or an export is one entry with the number of accounts and the file's SHA-256. |
| Roles and permissions | A role created or deleted, a permission added to a role or removed from it, a permission created or deleted. |
| Tenant settings | Changes saved in the Settings tabs (voting channels, languages, localization, integrations, look and feel, countries), election types, trustees and previews, and exporting or importing the tenant configuration in **Backup / Restore**. |
| Templates | Created, edited, deleted, imported and exported. |
| Election events | Created, imported, archived, unarchived and deleted. The entry of a deleted event keeps the last entry and the state hash of that event's log. |
| Changes made in Keycloak | Any change to the tenant's realm made in Keycloak's admin console or through its API. Changes the Admin Portal makes in Keycloak through its service account also appear, as SYSTEM entries next to the administrator's own entry. |
| Administrator requests | Every request an administrator makes to the platform's API outside an election event: creating, importing and deleting an election event; creating, editing, deleting, importing and exporting staff accounts; creating and deleting roles and permissions, giving or removing a role and adding or removing a permission; creating and deleting a tenant, importing and exporting its configuration, importing templates and limiting access by countries. A request is recorded whether or not it was allowed. |
| The log itself | Every export of the log, the log writer starting and stopping, and each administrator's signing key the first time it is used. |

An administrator request is one `TenantAdminAction` entry with the action, who asked, how the
request ended (accepted, denied or failed, with its HTTP status) and what it named, such as the
election event's ID or the role. Its **Log Type** is `INFO` when the platform accepted it and `ERROR`
when it was denied or could not be completed, so a refused attempt to delete an election event is as
visible as a successful one. "Accepted" means the platform took the request: long operations such as
an import continue as a task, which the **Tasks** list follows.

A sign-in code Keycloak sends to an account is recorded as a `KeycloakUserEvent` entry with the event
`MESSAGE_SENT`, without the code.

Everything else about one election event (its elections, contests, candidates, areas, voters,
publications, voting periods, tallies) goes to that event's log.

A Keycloak event that names an election event the platform cannot find, for example one that was
deleted, is written to the tenant log as an ERROR entry instead of being lost.

## Who can see it

Settings opens with the `settings-menu` permission. Its tabs then depend on two more:

| Permissions held, besides `settings-menu` | Settings shows |
|---|---|
| `tenant-write` and `tenant-logs-read` | Every tab, with **Logs** last |
| `tenant-write` only | Every tab except **Logs** |
| `tenant-logs-read` only | Only **Logs** |
| Neither | "You don't have permission to access settings." |

The **Messaging** tab also needs `messaging-account-read`, as before.

The **Export** button in the tab needs `tenant-logs-export` as well as `tenant-logs-read`: an export
is a copy of the log, so it is refused (403) to anyone who cannot read it. The server checks both permissions
again on every request, so the tenant log cannot be read or exported through the API without them.
New tenants give both permissions to the `admin` group; see
[Permissions](../users-and-roles/users-and-roles_permissions.md#tenant-log-permissions).

## The list

Entries are listed newest first. Select a row to open the entry.

| Column | Content |
|---|---|
| **Id** | The entry's number in the log. |
| **User Id** | The account's identifier. Hidden until you turn it on under **Columns**. |
| **Username** | The account the entry is about, or `-` for an entry without one. |
| **Created** | When the entry was written to the log. |
| **Statement Timestamp** | When the action happened. |
| **Statement kind** | The kind of entry; see the table below. |
| **Event Type** | `USER` for something a person did, `SYSTEM` for something the platform did. |
| **Log Type** | `INFO` or `ERROR`. Failed sign-ins and lockouts are `ERROR`. |
| **Description** | One line that says what happened, such as `Updated tenant "acme": settings.logs_timezone`. |
| **Message** | The whole signed entry as text. Hidden until you turn it on under **Columns**. |

Times show in the tenant log's timezone, with your own time below when it differs.

| Statement kind | Written for |
|---|---|
| `KeycloakUserEvent` | Sign-ins and account security |
| `AdminChange` | A change made in the Admin Portal, with its old and new values |
| `TenantAdminAction` | An administrator's request outside an election event, with its outcome |
| `KeycloakAdminEvent` | A change made in Keycloak |
| `LogsExported` | An export of this log |
| `LogWriterStarted`, `LogWriterStopped` | The log writer starting and stopping |
| `AdminPublicKey` | An administrator's signing key |

The list adapts to its width. Below 1,500 pixels **Created** starts hidden under **Columns**. Below
1,000 pixels each entry takes two lines and **Columns** is not offered; selecting an entry opens the
same popup.

### Filters

**Created from**, **to** and **Timezone** are always shown. The range includes both ends, to the
minute, and is read in the timezone chosen next to it: the tenant log's timezone or your own.

**Add filter** offers:

- **User Id** and **Username**;
- **Statement Timestamp from** and **Statement Timestamp to**;
- **Statement kind**, chosen from the kinds in the table above;
- **Event Type**: `USER` or `SYSTEM`;
- **Log Type**: `INFO` or `ERROR`.

For example, **Statement kind** `KeycloakUserEvent` with **Log Type** `ERROR` lists the failed
sign-ins and lockouts of the chosen range.

## The entry popup

Selecting an entry opens a popup titled **Log entry** and its number. **Close** or the Escape key
dismisses it. It starts with the description and three labels (the statement kind, the event type and
the log type), and then shows the sections that apply to the entry:

| Section | Content |
|---|---|
| Facts | **When**: the time in the log's timezone, your own time when it differs, and UTC. **Who**: the person and account, the user ID and the role. **Made in**: where the change was made (the Admin Portal, through Hasura, Harvest or a Windmill task). **Changed**: the kind of object, its name and ID. **Action**: Created, Updated, Deleted, Imported, Exported, Archived, Unarchived, Enabled, Disabled, Password reset, Granted or Revoked. |
| Request | For a `TenantAdminAction` entry, one line each for the action, how the request ended with its HTTP status, the account that asked, and each thing the request named. |
| **Old and new values** | One row per changed field, with **Field**, **Old value** and **New value**. Known fields carry a readable name above their path; other fields show the path alone. |
| **Sign-in and account security** | The **Keycloak event**, the **Error**, the **Authentication** and **Credential** used, the **Failures in a row**, the **Client** and the **IP address**, followed by the other details Keycloak gave. |
| **Change in Keycloak** | **Made in** (Keycloak's admin console, or Step's service account for a change made in the Admin Portal), the **Account** that made it with its client and IP address, the **Operation**, what it **Applies to**, the **Resource**, an **Error** if it failed, and the **New value** Keycloak stored. |
| **The deleted election event's log** | The **Last entry** of the event's log, its **State hash**, the **Log database** it was kept in, its **Status**, the **Task** that deleted the event and **The log's system key**, kept here because the event's keys are deleted with it. |
| **The imported file** | For an import: whether the file matches the **Declared checksum** the administrator typed (**Matches the file**, **Doesn't match the file** or **None declared**), the **SHA-256 of the file** the task read, what the file held (elections, contests, candidates, areas and areas per contest for an election event; the configuration sections for a tenant configuration; rows for staff accounts) and the **Document**. |
| **Export** | The **Format**, **Scope**, **Username**, range and **Timezone** of an export, its number of **Entries**, the **SHA-256 of the file** and the **Document**. |
| **Log writer** | The **Release** that runs and the **Last entry posted**. |
| **Details** | Other facts of the entry, such as the number of rows and the file's SHA-256 of an import. |
| **Log and signatures** | The **Log** the entry belongs to, who it was **Signed by** (the person's admin key, or "The log's system key only"), the **System signature** and, for an administrator's key, the **Published admin key**. Long values are shortened; hover over one to read it whole. |

A failed sign-in for an account that does not exist shows **Who** as "Unknown account (the name
typed is not recorded)". The log keeps neither the username nor the email that was typed.

### Secrets and long texts

- **Secrets are never logged.** When a secret changes, such as a password, an API key, a token or a
  private key, its row says "Changed. Secret values are never logged." instead of the old and new
  values. A field counts as a secret by its name.
- **Long texts are logged as hashes.** When an old or new value is longer than 64 KB, such as a large
  template body, the row shows `SHA-256` and the hash of each value instead of the text.
- A value that is not set shows as "(none)".

## Exporting

**Export** opens the **Export logs** dialog.

| Field | Choices |
|---|---|
| **Scope** | **The whole tenant log**, or **One account**. One account needs its **Username**, exactly as the list shows it. |
| **Include this account's entries in every election event** | Offered for one account. Adds that account's entries from the log of every election event of the tenant, deleted ones included, merged with the tenant log's by time. Each row names its log; a deleted event's rows name it as `retired event <ID>`. Its log is opened read-only for the export and closed again. |
| **From**, **To** | Optional. The range of **Created**, both ends included, to the minute. |
| **Timezone** | The timezone the range is read in and the file's local times are written in: the tenant log's timezone or your own. |
| **Format** | **CSV**, **SQL** or **PDF**. |

The export runs as an **Export Tenant Logs** task, which gives the file when it completes. The file
holds the entries the list shows for the same scope and range.

| Format | The file contains |
|---|---|
| CSV | One row per entry: the log it comes from (`tenant`, or `event` and the election event's ID), ID, created and statement timestamps, statement kind, event type, log type, user ID, username, description, the signed entry as JSON, the signed statement, the signer's name and public key, both signatures, and the created time in UTC and in the chosen timezone with the timezone's name. |
| SQL | A PostgreSQL script that creates the table `audit_log` with the same columns and inserts one row per entry, and a table `audit_log_export` with what the export holds: its scope, range, timezone, number of entries and the public key of each log's system key. An auditor can load it and check every signature; see [Verify a tenant log export](../../../../04-election_auditors/02-tutorials/07-auditor_verify-tenant-log-export.md). |
| PDF | A summary (scope, account, range, timezone, number of entries) and a table with the log, ID, created time in the chosen timezone and in UTC, username, statement kind, event type, log type and description of each entry. It does not include signatures. |

**The export itself is recorded in this log** as a `LogsExported` entry with the format, scope,
range, number of entries and the SHA-256 of the file. To check later that a file is the one that was
exported, compare its SHA-256 with that entry's.

## Timezone for tenant logs

Entries are stored in UTC. The tab shows them in the tenant log's timezone, set in **Settings** >
**Languages** > **Timezone for tenant logs**. It is UTC until someone chooses another. The setting
changes how times are shown and the default timezone of filters and exports; it does not change any
entry. Changing it is itself logged, as a change to tenant settings.
