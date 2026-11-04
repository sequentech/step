---
id: about
title: About the Handbook for Election Administrators
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

| Item | Value |
|---|---|
| Document ID | AGD-02 |
| Title | Handbook for Election Administrators |
| TOE reference | Sequent Voting Platform - uniWAHL Version, v10.0.0 |
| Document version | 1.1 |
| Software release | v10.0.0 |

## Who this guide is for

This handbook is for the administrators: the people whom the election organizer
appoints to set up and look after the TOE ([Roles and people](#roles-and-people)).
An administrator:

- configures the tenant;
- sets up the tenant's policy for signed configuration packages, checks the
  administrator settings on the election event's tab "Data" and obtains the
  board's check of the other imported settings against the required values: as
  administrator you see an election event's security settings, not its election
  data or its audit records;
- manages the user accounts and roles of the tenant, and keeps the
  administrator role and the election board role apart;
- sets the protected actions and their required authorisations, the automatic
  self-tests and the attack-detection thresholds, and gives the election
  organizer the required number of allowed votes (re-voting), which the board
  checks;
- acts on the TOE's notifications and supports the election board when a
  failure occurs;
- as the operator, installs and runs the servers of the TOE as the Deployment
  Guide describes, unless the election organizer gives these duties to another
  administrator.

The administrator does not carry out the election board's functions:
importing, checking and approving election data, publishing configuration
versions, authorising the security-critical actions, starting, interrupting,
resuming and ending the election execution, counting the votes and determining
the election result, reviewing the audit records, and exporting, archiving and
deleting election event data. The Handbook for
Election Organizers describes them.

## This document

This handbook is the operational user guidance for the **administrator** role
of the TOE. It applies to the evaluated configuration only: the one
configuration in which all components of the TOE are deployed as the
Deployment Guide describes and configured as this handbook describes. Do not
run an election event with any other configuration.

The administrator's work starts when the deployment is complete: the delivery
has been accepted, the operational environment and the TOE have been installed
and hardened, and the first personal administrator account works, all as the
Deployment Guide describes.

The admin portal is a supporting component outside the TOE. Its screens guide
the workflow; displaying or hiding a control is not a security boundary or
evidence of a server-side refusal. Follow the checks that this handbook gives for
each operation, including required authorisations.

## Roles and people

The TOE knows three roles: administrator, election board and voter. These guides
use these names for people:

| Name | Who this is | Role in the TOE |
|---|---|---|
| election organizer | The organisation that holds the election event. It appoints the administrators and the election board and acts in the TOE only through them. | none |
| administrator, or election administrator | The same person under two names: someone the election organizer appoints to set up and look after the TOE. The Handbook for Election Administrators (AGD-02) is written for them. Never a board member or a trustee. | administrator: the role `admin` in the admin portal |
| operator | An administrator in their duties on the servers: installing and running the servers, the certificates and the TOE images, as the Deployment Guide (AGD-04) describes. The election organizer may give these duties to the person who works in the admin portal or to someone else. Either way the operator counts as an administrator and is never a board member or a trustee. | administrator, on the servers |
| election board | The group of people the election organizer appoints to run the election event: it imports and approves the election data, starts and ends voting, counts the votes and reviews the audit records. | election board |
| board member | One member of the election board. | election board: the role `election-board` |
| trustee | A board member who also holds a share of the election key and takes part in the key ceremony and the counting. | election board: the roles `election-board` and `trustee` |
| voter | A person in the voters' register. | voter |
| observer | Anyone who checks the result independently with the election verifier. | none |

## Chapters of this handbook

| Chapter | Content |
|---|---|
| [The Administrator Role](02-administrator_role.md) | What the administrator may and may not do; the TOE and its supporting components; the privileges to control |
| [Secure Use of the Admin Portal](03-admin_portal.md) | Signing in, confirming sensitive actions, sessions and logout, and the version check |
| [Users, Roles and Role Separation](04-users_and_roles.md) | Accounts, the administrator role, the election board role, the trustee role, and what to do when a person leaves |
| [Tenant and Election Event Settings](05-tenant_and_event_settings.md) | Signed configuration packages, the administrator's part around the election board's import, each security setting of the election event and its elections with the required values, and the hand-over to the board for the publication of the configuration version |
| [Configuring Protected Actions](06-signing_rules.md) | Security-critical actions, required authorisations, trusted issuers and certificate checks |
| [Security and Self-Tests](07-self_tests.md) | The six background checks, configurable interval, security state, notification delivery and the check of the TOE software |
| [Authentication and Session Settings](08-authentication_and_sessions.md) | The external authentication server, the authentication failure threshold, session limits and the functions available before login |
| [Audit, Monitoring and Notifications](09-audit_and_monitoring.md) | The electoral log and the election board's review of it, attack detections, the audit storage threshold and email notifications |
| [Modes of Operation and Failure Handling](10-modes_of_operation.md) | The electoral phases, lockdown, maintenance mode, unrecoverable failures, the audit storage threshold and backups |
| [Security-Relevant Events for Administrators](11-security_relevant_events.md) | The security-relevant events per phase and what the administrator does for each |
| [Environment Measures and Security Problems](12-environment_and_security_problems.md) | The administrator's measures for the operational environment, and how to report a security problem |

## Other handbooks you need

- [Deployment Guide (AGD-04)](../../05-reference/06-deployment_guide/01-about.md):
  acceptance of the TOE delivery, installation of the operational environment
  and of the TOE, hardening, the initial accounts and the commands the
  operator uses on the servers. The operator completes it before you use this
  handbook.
- [Handbook for Election Organizers (AGD-03)](../05-election_organizer_handbook/01-about.md):
  the duties of the election organizer and the procedures of the election
  board and the trustees. Read it to know what the election board expects from
  your configuration.
- [Handbook for Voters (AGD-01)](../../03-voters/02-voter_handbook/01-about.md): the
  voters' guidance. Read it to know what voters see, so that you can support
  the election organizer.

## Terms

This handbook uses the terms of the Security Target, with one exception: what
the Security Target calls an election is called an election event here, as in
the admin portal. Where the admin portal uses another name for a term, the table
gives it. The [Roles and people](#roles-and-people) table defines the names of
people.

| Term in this handbook | What it means, and its name in the admin portal |
|---|---|
| tenant | the organisation's space in the TOE; it holds the administrators', board members' and trustees' accounts and the election events |
| realm | the TOE's set of accounts, roles and login settings. The tenant has one, and each election event has its own |
| election event | what the Security Target calls an election: the whole vote that the election organizer holds. An election event contains elections, an election contains contests, and a contest contains candidates |
| election data | the voters' register (the voters of the election event, with its areas), the candidate list and the display data of the ballot, the set of election rules (the counting and voting rules of the contests) and the election dates (the scheduled opening and closing of voting, with the end of the election period). The signed package also contains event and election Data-tab settings, including presentation and re-voting settings, which administrators and the election board verify after import, each in their own view of the tab "Data". Some of these settings affect the approved ballot design and must not be changed to repair an imported package. Administrators separately configure the rules and required authorisations of the Signatures tab, certificate registrations, the automatic self-tests and the attack-detection thresholds |
| election execution data | the cast votes, the electoral log, the tally and results data, and the voting records |
| electoral log | the audit records of one election event. Each election event has its own electoral log; there is none for the tenant as a whole. What concerns the whole tenant, such as a change of the attack-detection settings or one of the tenant's accounts reaching the authentication failure threshold, is recorded in the electoral log of every election event of the tenant. The election board reads an election event's electoral log in the tab "Logs" of the election event and in the side menu "Electoral Log"; administrators do not read it |
| audit records | the records of the electoral log |
| operating logs | the logs of the servers and containers, which the operator reads to run the system. They are not audit records |
| the board's minutes | the election board's own record of the election event, kept outside the TOE in the form the electoral regulation sets ([Write the electoral regulation](../05-election_organizer_handbook/02-organizer-duties.md#write-the-electoral-regulation)) |
| intermediate ballot box | the ballot boxes that receive the cast votes during the execution phase |
| ballot box | the anonymised boards of the bulletin board that the trustees mix and decrypt when the votes are counted |
| security-critical action | protected action (the election event's tab "Signatures", area "Protected actions") |
| required authorisations | the number in the field "Signatures needed" of the protected action's signing rule; for an import of election data, the number of package approvals that the tenant requires |
| electoral phase | the preparation, execution, evaluation or post-processing phase ([The electoral phases](10-modes_of_operation.md#the-electoral-phases)); the election event's tab "Security and self-tests" shows the current one |
| configuration package | the signed file of election data that the election board imports |
| configuration version | a published version of the election event's configuration, which the election board approves |
| approval event | an existing election event in the preparation phase in which the election board approves an import ([The approval event](05-tenant_and_event_settings.md#the-approval-event)) |
| approved baseline | the TOE's signed record of the election data and the access-control settings at the election board's approval of a configuration version. The self-tests compare against it. On screen: "board-approved configuration baseline" |
| lockdown period | the period the election board sets during which the configuration of the election event cannot change |
| maintenance mode | the state after a recoverable failure, on screen "Recovery required" (a new election event shows the same words until voting first opens, [Maintenance mode](10-modes_of_operation.md#maintenance-mode)): voting is stopped until the election board resumes it after a passing self-test |
| unrecoverable failure | the state after a failure that cannot be recovered from: the election execution ends and voting never resumes |
| role | a role in Users and Roles: a named set of permissions given to accounts |
| board permission | a permission that only the election board role holds |
| re-voting | voting again, where only the last vote counts; the field "Number of allowed votes" of an election sets how often a voter may vote |

## Conventions

Screenshots illustrate real interfaces with synthetic test data. Sample
accounts, election event and election names, identifiers, hashes and dates are not values to copy
into an election event. Follow the steps and required values in the text; a pictured
screen is not evidence that your own operation completed.

- Names in quotation marks or in bold, such as "Users and Roles" or
  **Run self-tests**, are the exact texts of the admin portal in English: menu
  entries, tabs, buttons, fields, options, dialogs and messages.
- A path such as Settings > "Attack detection" means: open Settings in the side
  menu, then the tab named "Attack detection".
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

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-10-27 | First edition for the evaluated configuration of v10.0.0. |
| 1.1 | 2026-11-02 | Numbered sections; consistent use of election event; all chapters revised; new screenshots. |
