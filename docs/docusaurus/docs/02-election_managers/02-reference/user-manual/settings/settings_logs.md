---
id: settings_logs
title: Logs
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Logs** tab of **Settings** shows the tenant's log: what happens in the
organization outside any election event. It complements the **Logs** tab of each election
event, which keeps what happens inside that event. Together they record the platform's
activity from the first sign-in to the last.

## What the log records

| Statement kind | Recorded when | What the entry holds |
|---|---|---|
| `TenantLogOpened` | The log is created, the first time the tenant has something to record. | The name of the log and the public key its entries are signed with. |
| `KeycloakUserEvent` | Someone signs in to the Admin Portal, fails to, signs out, or changes a password or an authenticator. | The Keycloak event type, such as `LOGIN` or `LOGIN_ERROR`, the error code of a failure, and the account. A failed sign-in for an account that doesn't exist is recorded as `unknown_user`. A code sent to an account is recorded as `MESSAGE_SENT`, without the code. |
| `TenantAdminAction` | An administrator asks the platform for something that belongs to no election event. | The action, who asked, how the request ended and what it was made on. |

The administrator requests recorded are:

- **Election events:** creating, importing and deleting one.
- **Staff accounts:** creating, editing, deleting, importing and exporting them. Voters belong
  to an election event, so changes to voters stay in that event's log.
- **Roles and permissions:** creating and deleting a role or a permission, giving a role to
  an account or removing it, and adding a permission to a role or removing it.
- **The tenant:** creating and deleting a tenant, importing and exporting its configuration,
  importing templates, and limiting access by countries.

A request is recorded whether or not it was allowed. The **Log Type** column is `INFO` for a
request the platform accepted and `ERROR` for one it denied or could not complete, so a
refused attempt to delete an election event is as visible as a successful one. "Accepted"
means the platform took the request: long operations such as an import continue as a task,
which the **Tasks** list follows.

Token renewals of a signed-in administrator are not recorded: they are not something the
administrator did.

## Reading the log

The list shows the newest entries first, with the time of each in your timezone.

- **Username** and **Statement kind** say who did what kind of thing; **Description** says it
  in a sentence, such as "Request to delete an election event denied."
- **Show More** in the description of a request opens its details: the action, the outcome
  with its HTTP status, the account that asked, and what the request named, such as the
  election event's id or the role.
- **Add filter** narrows the list by username, user id, statement kind or statement
  timestamp. The **Created from** and **to** filters are always shown and include both ends,
  to the minute.
- **Columns** adds the user id, the time the entry was stored and the signed message.

Every entry is signed with the key of the tenant's log and appended to a Merkle log, as the
entries of an election event's log are. An entry cannot be edited or deleted from the
Admin Portal.

## Who can read it

The tab needs the `tenant-logs-read` permission, and Settings needs `settings-menu`. It
doesn't need `tenant-write`: a role with `settings-menu` and `tenant-logs-read` and without
`tenant-write`, such as an auditor's, opens Settings and sees only **Logs**. A role with
`tenant-write` and without `tenant-logs-read` sees every other tab and not this one.

## What it does not record yet

- Changes saved from the other Settings tabs (voting channels, languages, look and feel and
  the rest) are written to the database directly and are not in the tenant's log.
- The tenant's log has no export and no audit action. The entries can be read and filtered
  in this tab.
- A tenant created before this feature gets its log the first time it has something to
  record. Its realm must list the `custom-event-listener` event listener for sign-ins to be
  recorded; see the [tenant log](../../../../07-developers/13-electoral-log/04-tenant-log.md)
  developer page.
