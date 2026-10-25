---
id: about
title: About the Handbook for Election Administrators
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## This document

| Item | Value |
|---|---|
| Document ID | AGD-02 |
| Title | Handbook for Election Administrators |
| TOE reference | Sequent Voting Platform - uniWAHL Version, v10.0.0 |
| Document version | 1.0 |
| Software release | v10.0.0 |

This handbook is the operational user guidance for the **administrator** role
of the TOE. It applies to the evaluated configuration only: the one
configuration in which all components of the TOE are deployed as the
Deployment Guide describes and configured as this handbook describes. Do not
run an election with any other configuration.

The administrator's work starts when the deployment is complete: the delivery
has been accepted, the operational environment and the TOE have been installed
and hardened, and the first personal administrator account works, all as the
Deployment Guide describes.

## Who this handbook is for

This handbook is for the persons whom the election organizer appoints as
administrators. An administrator:

- configures the tenant;
- sets up the tenant's policy for signed configuration packages and verifies
  the imported event and election Data-tab settings against the required values:
  as administrator you see an election event's settings, not its election data
  or its audit records;
- manages the user accounts and roles of the tenant, and keeps the
  administrator role and the election board role apart;
- sets the protected actions and their required authorisations, the self-test
  settings and realm authentication settings, and checks the imported re-voting
  setting;
- acts on the TOE's notifications and supports the election board when a
  failure occurs;
- operates the servers of the TOE as the Deployment Guide describes.

The administrator does not carry out the election board's functions:
importing, checking and approving election data, publishing configuration
versions, authorising the security-critical actions, starting, interrupting,
resuming and ending the election execution, counting the votes and determining
the election result, reviewing the audit records, and exporting, archiving and
deleting election event data. The Handbook for
Election Organizers describes them.

## Pages of this handbook

| Page | Content |
|---|---|
| [The Administrator Role](02-administrator_role.md) | What the administrator may and may not do; the TOE and its supporting components; the privileges to control |
| [Secure Use of the Admin Portal](03-admin_portal.md) | Signing in, confirming sensitive actions, sessions and logout, the version display and the image check |
| [Users, Roles and Role Separation](04-users_and_roles.md) | Accounts, the administrator role, the election board role, the trustee role, and what to do when a person leaves |
| [Tenant and Election Event Settings](05-tenant_and_event_settings.md) | Signed configuration packages, the administrator's part around the election board's import, each security setting of the election event and its elections with the required values, and the hand-over to the board for the publication of the configuration version |
| [Configuring Protected Actions](06-signing_rules.md) | Security-critical actions, required authorisations, trusted issuers and certificate checks |
| [Self-Test Settings](07-self_tests.md) | When the TOE tests itself, what it tests, the settings and the secure interval values |
| [Authentication and Session Settings](08-authentication_and_sessions.md) | The external authentication server, the authentication failure threshold, session limits and the functions available before login |
| [Audit, Monitoring and Notifications](09-audit_and_monitoring.md) | The electoral log and the election board's review of it, attack detections, the audit storage threshold and email notifications |
| [Modes of Operation and Failure Handling](10-modes_of_operation.md) | The electoral phases, lockdown, maintenance mode, unrecoverable failures, the audit storage threshold and backups |
| [Security-Relevant Events for Administrators](11-security_relevant_events.md) | The security-relevant events per phase and what the administrator does for each |
| [Environment Measures and Security Problems](12-environment_and_security_problems.md) | The administrator's measures for the operational environment, and how to report a security problem |

## Other handbooks you need

- [Deployment Guide](../../05-reference/06-deployment_guide/01-about.md):
  acceptance of the TOE delivery, installation of the operational environment
  and of the TOE, hardening, the initial accounts and the commands the
  administrator uses on the servers. Complete it before you use this handbook.
- [Handbook for Election Organizers](../05-election_organizer_handbook/01-about.md):
  the duties of the election organizer and the procedures of the election
  board and the trustees. Read it to know what the election board expects from
  your configuration.
- [Handbook for Voters](../../03-voters/02-voter_handbook/01-about.md): the
  voters' guidance. Read it to know what voters see, so that you can support
  the election organizer.

## Terms

This handbook uses the terms of the Security Target. Where the admin portal
uses another name, the table gives it.

| Term in this handbook | Name in the admin portal |
|---|---|
| election | election event (an election event contains elections, an election contains contests, a contest contains candidates); the Signatures tab calls an election a "Post" |
| tenant | the organisation's space in the TOE; it holds the administrators', board members' and trustees' accounts and the election events |
| election data | the voters' register (the voters of the election event, with its areas), the candidate list and the display data of the ballot, the set of election rules (the counting and voting rules of the contests) and the election dates (the scheduled opening and closing of voting, with the end of the election period). The signed package also contains event and election Data-tab settings, including presentation and re-voting settings, which administrators verify after import. Some of these settings affect the approved ballot design and must not be changed to repair an imported package. Administrators separately configure the rules and required authorisations of the Signatures tab, certificate registrations, self-test settings and realm authentication and session settings |
| election execution data | the cast votes, the electoral log (Logs tab, Electoral Log console), the tally and results data, and the voting records |
| intermediate ballot box | the ballot boxes that receive the cast votes during the execution phase |
| ballot box | the anonymised boards of the bulletin board that the trustees mix and decrypt when the votes are counted |
| security-critical action | protected action (Signatures tab, "Protected actions") |
| required authorisations | "Signatures needed" of the protected action's signing rule |
| electoral phase | "Electoral phase" on the election event's Dashboard: preparation, execution, evaluation or post-processing phase |
| audit records | electoral log records |
| role | a role in Users and Roles: a named set of permissions given to accounts |
| re-voting | "Number of allowed votes" of an election |

## Conventions

- Names in quotation marks, such as "Users and Roles", are the exact texts of
  the admin portal in English: menu entries, tabs, buttons, fields, options,
  dialogs and messages.
- A path such as Settings > "Backup / Restore" means: open Settings in the side
  menu, then the tab named "Backup / Restore".
- Permissions are given with their label in Users and Roles and their
  technical name, for example "Edit Election State" (`election-state-write`).
- File names and values to type are shown in code format. Inside a code block,
  a lower-case word between angle brackets stands for a value you supply.
- Admonitions mark what needs attention:
  - a **warning** box says what goes wrong, or which security property is
    lost, if you do not follow the instruction;
  - a **danger** box marks an action that can destroy election data or
    election execution data, or disclose secret data;
  - an **info** or **note** box gives background.

## Document history

| Release | Changes |
|---|---|
| v10.0.0 | First edition for the evaluated configuration. |
