---
id: administrator_role
title: The Administrator Role
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## The administrator in the TOE

The TOE knows three roles: **administrator**, **election board** and
**voter**. The election organizer appoints the administrators and the members
of the election board. Trustees are members of the election board who hold a
share of the election key.

An administrator is a person with a personal account in the tenant that holds
the administrator role and no permission of the election board role
([Users, Roles and Role Separation](04-users_and_roles.md)). The administrator
signs in at the external authentication server of the organisation, with two
factors.

The Security Target requires that administrators are trustworthy, instructed in
the correct handling of the TOE, do not intentionally misuse the TOE's security
functions, and close their sessions when they are not actively working with the
TOE. This handbook is that instruction.

## The TOE and the supporting components

The TOE consists of seven components:

| Component | What it does |
|---|---|
| harvest | the central backend: access control, voters' register, ballot boxes, import and export |
| windmill | the worker that runs the TOE's background tasks, such as imports, exports, sealing, audits and scheduled events |
| keycloak | identity and access management: accounts, roles, sessions and the connection to the external authentication server |
| ballot-verifier | the application voters use to audit an encrypted ballot before casting |
| election-verifier | the tool that verifies the proofs of the mixing and decryption of the votes, run on demand on exported data |
| braid | the trustee component that generates the election key, mixes and decrypts the votes |
| b4 | the bulletin board of the trustees |

The evaluated configuration also contains **supporting components** of the
operational environment, which are not part of the TOE: the admin portal and
voting portal web applications, the TLS endpoint and the ingress in front of the TOE,
the GraphQL engine that executes the TOE's access rules, the database, the
document storage, the container platform and operating system of the servers,
and the time service. The Deployment Guide installs and hardens them.

You use the TOE through the **admin portal**, a supporting web application in
your browser. The admin portal shows only the functions your permissions allow,
but that is a convenience: the TOE checks every request on the server side,
whatever the admin portal shows.

## What the administrator does

| Task | When | Page |
|---|---|---|
| Set up the tenant's signed configuration packages | once, before the first election event is imported; afterwards only a staff CA is replaced, between elections | [Tenant and Election Event Settings](05-tenant_and_event_settings.md) |
| Create the administrator, election board and trustee roles and the personal accounts; delete the seeded account `admin` | after deployment; whenever a person joins or leaves | [Users, Roles and Role Separation](04-users_and_roles.md) |
| Verify the imported event and election Data-tab settings against the required values; complete the separate administrator settings and tell the board when it can publish the configuration version | preparation phase, after each import | [Tenant and Election Event Settings](05-tenant_and_event_settings.md) |
| Set the protected actions and their required authorisations | preparation phase | [Configuring Protected Actions](06-signing_rules.md) |
| Agree the background audit interval with the operator and review the missing complete self-test requirements | preparation phase | [Self-Test Settings](07-self_tests.md) |
| Set the attack detection rates | preparation phase | [Audit, Monitoring and Notifications](09-audit_and_monitoring.md#attack-detections) |
| Give the election organizer the required number of allowed votes (re-voting), and verify the imported value for each election | before package approval and after import | [Tenant and Election Event Settings](05-tenant_and_event_settings.md) |
| Set the authentication failure threshold and check the session and login settings | after deployment, and for each new election event | [Authentication and Session Settings](08-authentication_and_sessions.md) |
| Act on the TOE's notifications and on what the election board reports, and support the board's audit review | every phase, especially the execution phase | [Audit, Monitoring and Notifications](09-audit_and_monitoring.md) |
| Support the election board in maintenance mode and after failures | when they occur | [Modes of Operation and Failure Handling](10-modes_of_operation.md) |
| Check the version of the TOE and its images | after deployment and after each update | [Secure Use of the Admin Portal](03-admin_portal.md) |
| Operate and back up the servers | every phase | the Deployment Guide |

## Functions and privileges to control

The administrator's permissions are powerful. The following functions change
the security of the election. Use them only for the purpose stated, only in the
phase stated, and only as the election organizer has decided.

| Function | Permissions | Why it must be controlled |
|---|---|---|
| Accounts and roles | "Create User" (`user-create`), "Edit User" (`user-write`), "Create Role" (`role-create`), "Edit Role" (`role-write`), "Edit User Permission" (`user-permission-write`), "Edit Permission Label" (`permission-label-write`) | Whoever holds them can give any account any permission, including the election board's. The separation of the roles depends on how you use them. |
| Protected actions | "Signatures: edit protected actions" (`signing-rules-write`) | Sets which actions need the election board's authorisations and how many. |
| Trusted issuers and certificate checks | "Signatures: import and remove trusted issuers" (`signing-issuers-write`), "Signatures: edit certificate checks" (`signing-checks-write`), "Signatures: register certificates" (`signing-certificates-register`), "Signatures: revoke certificates" (`signing-certificates-revoke`) | Decide whose signatures the TOE accepts for protected actions. |
| Tenant configuration | "Edit Tenant" (`tenant-write`) | Includes the rules for signed configuration packages, which protect the integrity and authenticity of imported election data. |
| Election event and election settings | "Edit Election Event" (`election-event-write`), "Edit Election" (`election-write`) | Include the ballot box seal, receipts, verification period, grace period, tally, re-voting and voter authentication settings. The TOE shows the administrator only the settings in these forms, as listed in [Tenant and Election Event Settings](05-tenant_and_event_settings.md). Values that affect the approved ballot design must already be correct in the signed package: verify them after import, and request a corrected package instead of editing them to repair the import. |
| Permissions on election data and audit records | for example "Create Election Event" (`election-event-create`), "Read Voter" (`voter-read`), "Read Logs" (`logs-read`), "Read Publish" (`publish-read`), "Edit Scheduled Events" (`scheduled-event-write`), "Read Area" (`area-read`), "Delete Election Event" (`election-event-delete`) ([the full list](04-users_and_roles.md#permissions-on-election-data-and-audit-records)) | The administrator role holds none of them. Whoever holds them imports, reads, publishes or deletes election data, or reads the audit records. Never add one of them to the administrator role, and give "Edit Results Publication" (`publish-results-write`) to no role. |
| Configure background integrity audits | The operator sets Beat/Windmill environment values; there is no self-test permission or settings screen. | [Self-Test Settings](07-self_tests.md) |
| Attack detection rates | `attack-detection-write` | Settings, section "Attack detection": "Failed logins per minute, all accounts" and "Requests per minute, external interfaces" decide when the TOE reports an attack to the election board. |

:::warning
Never give yourself or another administrator a permission of the election
board role, and never sign, start, interrupt, resume or end the election
execution yourself. If an account holds both roles, one person can perform the
security-critical actions alone and the required authorisations no longer
protect the election. Every change of users, roles, role permissions and role assignments is
recorded in the electoral log, which the election board checks.
:::

:::warning
Change the security settings of an election event only in the preparation
phase. Several of them are locked once voting has started, and the TOE refuses
configuration changes while the election event is locked down; a change that is
still possible later can change the rules of an election that is running.
:::

:::warning
Never use the seeded accounts of a fresh deployment or the trustees' service
accounts to sign in, and never share an account or its second factor. The
TOE's records name the account, not the person: a shared account makes the
electoral log worthless as evidence.
:::

## What the administrator never does

- Approve election data, or sign any protected action.
- Start, interrupt, resume or end the election execution, or leave maintenance
  mode.
- Start the counting, determine or publish the election result, export
  election execution data, or archive or delete an election event.
- Import election data, read it, change it or publish it in a configuration
  version: a member of the election board imports it as a signed
  configuration package, and the board publishes it
  ([Tenant and Election Event Settings](05-tenant_and_event_settings.md)). As
  administrator you see an election event's settings, not its election data
  or its audit records.
- Read the cast votes, the voting records, the voters' register or the audit
  records (the electoral log). The election board reviews the audit records;
  you learn of events from the TOE's notifications and from the board.
- Switch on automatic key or tally ceremonies.
- Use any interface of the TOE other than the admin portal and the procedures
  of the Deployment Guide.

These are the election board's functions or are forbidden in the evaluated
configuration.
