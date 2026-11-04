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
the administrator role and no board permission
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

| Task | When | Chapter |
|---|---|---|
| Set up the tenant's signed configuration packages | once, before a board member creates the first election event; afterwards only an election board approval CA is replaced, while every election event of the tenant is archived or deleted | [Tenant and Election Event Settings](05-tenant_and_event_settings.md) |
| Check the administrator, election board and trustee roles; create the personal accounts; delete the seeded account `admin` | after deployment; whenever a person joins or leaves | [Users, Roles and Role Separation](04-users_and_roles.md) |
| Check the imported election event's administrator settings and obtain the board's check of the other settings against the required values; complete the separate administrator settings and tell the board when it can publish the configuration version | preparation phase, after each import | [Tenant and Election Event Settings](05-tenant_and_event_settings.md) |
| Set the protected actions and their required authorisations | preparation phase | [Configuring Protected Actions](06-signing_rules.md) |
| Configure the background self-test interval with the organizer; after the board's approval of the configuration version, verify all six checks | preparation phase | [Security and Self-Tests](07-self_tests.md) |
| Verify that the running TOE software matches the accepted delivery | after deployment, after each update, after a server component crashed and before the election board approves a configuration version | [Verify the TOE software](07-self_tests.md#verify-the-toe-software) |
| Set the attack detection rates and the authentication failure threshold | preparation phase | [Audit, Monitoring and Notifications](09-audit_and_monitoring.md#attack-detections) |
| Give the election organizer the required number of allowed votes (re-voting); the board checks the imported value of each election | before package approval and after import | [Tenant and Election Event Settings](05-tenant_and_event_settings.md) |
| Check the session and login settings | after deployment, and for each new election event | [Authentication and Session Settings](08-authentication_and_sessions.md) |
| Act on the TOE's notifications and on what the election board reports, and support the board's audit review | every phase, especially the execution phase | [Audit, Monitoring and Notifications](09-audit_and_monitoring.md) |
| Support the election board in maintenance mode and after failures | when they occur | [Modes of Operation and Failure Handling](10-modes_of_operation.md) |
| Check the version of the TOE | after deployment and after each update | [Secure Use of the Admin Portal](03-admin_portal.md) |
| The operator's duties: operate and back up the servers | every phase | the [Deployment Guide](../../05-reference/06-deployment_guide/01-about.md) |

## Functions and privileges to control

The administrator's permissions are powerful. The following functions change
the security of the election event. Use them only for the purpose stated, only in the
phase stated, and only as the election organizer has decided.

| Function | Permissions | Why it must be controlled |
|---|---|---|
| Accounts and roles | <ul><li>"Create User" (<code>user-create</code>)</li><li>"Edit User" (<code>user-write</code>)</li><li>"Create Role" (<code>role-create</code>)</li><li>"Edit Role" (<code>role-write</code>)</li><li>"Edit User Permission" (<code>user-permission-write</code>)</li></ul> | Use these permissions only for authorized account administration. The TOE refuses changes to your own role memberships and to roles that affect your own account, and an account with an administrator permission receives only administrator permissions in its tokens. The TOE cannot tell whether a change is authorized or whose a certificate is: have a second administrator check every change of roles, permissions and certificate registrations, and note the check on your copy of the [Checklist before the election board approves the configuration version](11-security_relevant_events.md#checklist-before-the-election-board-approves-the-configuration-version), which goes to the board's minutes. |
| Protected actions | "Signatures: edit protected actions" (`signing-rules-write`) | Sets which actions need the election board's authorisations and how many. |
| Trusted issuers and certificate checks | <ul><li>"Signatures: import and remove trusted issuers" (<code>signing-issuers-write</code>)</li><li>"Signatures: edit certificate checks" (<code>signing-checks-write</code>)</li><li>"Signatures: register certificates" (<code>signing-certificates-register</code>)</li><li>"Signatures: revoke certificates" (<code>signing-certificates-revoke</code>)</li></ul> | Decide whose signatures the TOE accepts for protected actions. |
| Trustees' public keys | "Edit Trustee" (`trustee-write`) | Decides which public keys the key ceremony uses for each trustee: register only keys checked with the trustee ([Registering the trustees' public keys](04-users_and_roles.md#registering-the-trustees-public-keys)). |
| Tenant configuration | "Edit Tenant" (`tenant-write`) | Includes the rules for signed configuration packages, which protect the integrity and authenticity of imported election data. |
| Election event administration settings | "Admin User" (`admin-user`) | You can change only the administrator settings on the election event's tab "Data", and only in the preparation phase. The TOE does not show you each election's own settings, and shows only the settings that [Tenant and Election Event Settings](05-tenant_and_event_settings.md) lists. Values that affect the approved ballot design must already be correct in the signed package: verify them after import, and request a corrected package instead of editing them to repair the import. |
| Permissions on election data and audit records | For example: <ul><li>"Create Election Event" (<code>election-event-create</code>)</li><li>"Read Voter" (<code>voter-read</code>)</li><li>"Read Logs" (<code>logs-read</code>)</li><li>"Read Publish" (<code>publish-read</code>)</li><li>"Edit Scheduled Events" (<code>scheduled-event-write</code>)</li><li>"Read Area" (<code>area-read</code>)</li><li>"Delete Election Event" (<code>election-event-delete</code>)</li></ul> | The TOE never gives them to an administrator account ([The administrator role](04-users_and_roles.md#the-administrator-role)). Whoever holds them imports, reads, publishes or deletes election data, or reads the audit records. Never give an administrator a role that holds them, and give "Edit Results Publication" (`publish-results-write`) to no role. |
| Configure background self-tests | <ul><li>"Self-tests: read results" (<code>self-test-read</code>)</li><li>"Self-tests: run checks" (<code>self-test-run</code>)</li><li>"Self-tests: edit settings" (<code>self-test-settings-write</code>), to change the interval and the optional conditions in preparation</li></ul> | [Security and Self-Tests](07-self_tests.md) |
| Attack detection rates and the authentication failure threshold | "Edit Tenant" (`tenant-write`) and "Admin User" (`admin-user`) | Settings, tab "Attack detection". "Failed logins per minute, all accounts", "Requests per minute, external interfaces" and "Failed sign-ins per account before an alert" decide when the TOE records a potential attack for the election board ([Attack detections](09-audit_and_monitoring.md#attack-detections)). |
| Tenant backup and restore | "Edit Tenant" (`tenant-write`) | Settings, tab "Backup / Restore", can import a tenant configuration, a Keycloak configuration and roles and permissions from a file, which can change what this handbook sets up. This handbook gives no procedure for it: do not import anything there. |

:::warning
Never give yourself or another administrator a board permission, and never
sign, start, interrupt, resume or end the election execution yourself. An administrator token cannot gain board capabilities by combining roles.
Still correct conflicting assignments and preserve distinct board identities;
a role assignment does not replace a registered certificate or board quorum. The election board sees changes of "Sign:" permissions in the electoral log. The electoral log does not record other changes of accounts and roles
([The electoral log](09-audit_and_monitoring.md#the-electoral-log)).
:::

:::warning
Change the security settings of an election event only in the preparation
phase. Several of them are locked once voting has started, and the TOE refuses
configuration changes while the election event is locked down; a change that is
still possible later can change the rules of an election event that is running.
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
