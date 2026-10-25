---
id: about
title: About the Handbook for Election Organizers
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## This document

| Item | Value |
|---|---|
| Document ID | AGD-03 |
| Title | Handbook for Election Organizers |
| TOE reference | Sequent Voting Platform - uniWAHL Version, v10.0.0 |
| Document version | 1.0 |
| Software release | v10.0.0 |

This handbook is the operational user guidance for the election organizer and
for the election board, including the trustees. It applies to the evaluated
configuration of the TOE only: one configuration, set up as the Deployment
Guide describes and configured as the Handbook for Election Administrators
describes. Do not run an election with any other configuration.

## Who this handbook is for

| Reader | What they do with the TOE | Pages |
|---|---|---|
| Election organizer | Hosts the election. Appoints the election board and the administrators, issues and distributes credentials, instructs voters, sets the electoral regulation. Acts in the TOE only through the administrators and the election board it appoints. | [Duties of the Election Organizer](02-organizer-duties.md) |
| Election board member | Approves the configuration packages of the election data and imports them; publishes and approves the configuration version and sets the lockdown period; starts, interrupts, resumes and ends the election execution; checks the audit records, counts the votes, exports the election execution data, archives a wrong import and deletes an election event after its export; each security-critical action needs the required authorisations. | All pages |
| Trustee | A board member who holds a share of the election key and takes part in the key ceremony and the counting. | [Trustee Procedures](09-trustee-procedures.md) |
| Observer | Checks the result independently with the election-verifier. | [Verifying the Result with the Election Verifier](10-election-verifier.md) |

## Other handbooks you need

- The Handbook for Election Administrators (AGD-02): the administrators'
  tasks: the settings of the tenant and the election events, roles and
  permissions, signing rules, self-test settings and the realm settings that
  this handbook relies on.
- The Deployment Guide (AGD-04): acceptance of the TOE delivery, installation
  of the operational environment and of the TOE, its hardening, and how
  trustees run the `trustee` program of the delivered `braid` image.
- The Handbook for Voters (AGD-01): the voters' guidance. The election
  organizer makes it available to the voters through its own channels.

## Terms

This handbook uses the following terms. Where the admin portal uses another
name for the same thing, the table gives it.

| Term in this handbook | Name in the admin portal |
|---|---|
| election | election event (an election event contains elections, an election contains contests, a contest contains candidates); the Signatures tab calls an election a "Post" |
| election data | the voters' register (the voters of the election event, with its areas), the candidate list and the display data of the ballot, the set of election rules (the counting and voting rules of the contests) and the election dates (the scheduled opening and closing of voting, with the end of the election period). The signed package also contains event and election Data-tab settings, including presentation and re-voting settings, which administrators verify after import. Some of these settings affect the approved ballot design and must not be changed to repair an imported package. Administrators separately configure the rules and required authorisations of the Signatures tab, certificate registrations, self-test settings and realm authentication and session settings |
| election execution data | the cast votes, the electoral log (Logs tab), the tally and results data, and the voting records |
| intermediate ballot box | the ballot boxes that receive the cast votes during the execution phase |
| ballot box | the anonymised boards of the bulletin board that the trustees mix and decrypt when the votes are counted |
| security-critical action | protected action (Signatures tab, "Protected actions") |
| required authorisations | "Signatures needed" of the protected action's signing rule; for an import of election data, the number of package approvals that the tenant requires |
| package approval | a board member's signature inside a signed configuration package, made with their approval certificate in the election organizer's package tool, outside the TOE, before the import |
| electoral phase | "Electoral phase" on the election event's Dashboard |
| audit records | electoral log records |
| re-voting | "Number of allowed votes" of an election |
| configuration hash and Tally ID of a tally batch | Full "Configuration hash" and decimal "Tally ID" beside that protocol board's "Download verification data" button in "Election verification", in the completed results tally on the Tally tab. This is the protocol-board identifier, not the tally-session ID. |
| operators | the administrators in their task of installing and running the TOE and its operational environment, as the Deployment Guide (AGD-04) describes |
| trustee server | the part of the TOE (`braid`) that takes part in the ceremonies for one trustee on the protected server, with that trustee's keys file; one instance per trustee |

## Roles

The TOE knows three roles: **administrator**, **election board** and
**voter**. The election organizer is not a role of the TOE: it acts through the
administrators and the election board it appoints. Trustees are members of the election board.

## Conventions

- Names in quotation marks, such as "Stop Voting", are the exact texts of the
  admin portal in English: tabs, buttons, fields, dialogs and messages.
- A path such as Settings > "TRUSTEES" means: open Settings in the side menu,
  then the tab named "TRUSTEES".
- Permissions are given with their label in Users and Roles and their
  technical name, for example "Edit Election State" (`election-state-write`).
- Commands are shown in code blocks. Inside a code block, a lower-case word
  between angle brackets stands for a value you supply.
- Admonitions mark what needs attention:
  - a **warning** box says what goes wrong, or which security property is
    lost, if you do not follow the instruction;
  - an **info** or **note** box gives background.

## Document history

| Release | Changes |
|---|---|
| v10.0.0 | First edition for the evaluated configuration. |
