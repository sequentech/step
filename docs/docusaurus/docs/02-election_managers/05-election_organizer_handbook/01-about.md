---
id: about
title: About the Handbook for Election Organizers
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

| Item | Value |
|---|---|
| Document ID | AGD-03 |
| Title | Handbook for Election Organizers |
| TOE reference | Sequent Voting Platform - uniWAHL Version, v10.0.0 |
| Document version | 1.1 |
| Software release | v10.0.0 |

## Who this guide is for

| Reader | What they do with the TOE | Chapters |
|---|---|---|
| Election organizer | Hosts the election event. Appoints the election board and the administrators, issues and distributes credentials, instructs voters, sets the electoral regulation. Acts in the TOE only through the administrators and the election board it appoints. | [Duties of the Election Organizer](02-organizer-duties.md) |
| Election board member | Approves the configuration packages of the election data and imports them; publishes and approves the configuration version and sets the lockdown period; starts, interrupts, resumes and ends the election execution; checks the audit records, counts the votes, exports the election execution data, archives a wrong import and deletes an election event after its export; each security-critical action needs the required authorisations. | All chapters |
| Trustee | A board member who holds a share of the election key and takes part in the key ceremony and the counting. | [Trustee Procedures](09-trustee-procedures.md) |
| Observer | Checks the result independently with the election verifier. | [Verifying the Result with the Election Verifier](10-election-verifier.md) |

## This document

This handbook is the operational user guidance for the election organizer and
for the election board, including the trustees. It applies to the evaluated
configuration of the TOE only: one configuration, set up as the Deployment
Guide describes and configured as the Handbook for Election Administrators
describes. Do not run an election event with any other configuration.

The admin portal is a supporting component outside the TOE. Its screens guide
the workflow; displaying or hiding a control is not a security boundary or
evidence of a server-side refusal. Follow the operation-specific checks and
completion evidence in this handbook, including required authorisations.

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

These guides also say "the board" for the election board and "the organizer" for
the election organizer.

## Other handbooks you need

- The Handbook for Election Administrators (AGD-02): the administrators'
  tasks: the settings of the tenant and the election events, roles and
  permissions, signing rules, the self-test settings and the realm settings that
  this handbook relies on.
- The Deployment Guide (AGD-04): acceptance of the TOE delivery, installation
  of the operational environment and of the TOE, its hardening, and how
  trustees run the `trustee` program of the delivered `braid` image.
- The Handbook for Voters (AGD-01): the voters' guidance. The election
  organizer makes it available to the voters through its own channels.

## Terms

This handbook uses the terms of the Security Target, with one exception: what
the Security Target calls an election is called an election event here, as in
the admin portal. Where the admin portal uses another name for a term, the table
gives it. The [Roles and people](#roles-and-people) table defines the names of
people.

| Term in this handbook | What it means, and its name in the admin portal |
|---|---|
| TOE | the target of evaluation: the evaluated parts of Sequent Voting Platform - uniWAHL Version, v10.0.0 |
| Security Target | the document that states what was evaluated |
| tenant | the organisation's space in the TOE; it holds the administrators', board members' and trustees' accounts and the election events |
| election event | what the Security Target calls an election: the whole vote that the election organizer holds. An election event contains elections, an election contains contests, and a contest contains candidates |
| election data | the voters' register (the voters of the election event, with its areas), the candidate list and the display data of the ballot, the set of election rules (the counting and voting rules of the contests) and the election dates (the scheduled opening and closing of voting, with the end of the election period) |
| other settings of the election event | the event and election settings of the tab "Data", among them presentation and re-voting settings, which the signed package contains and administrators and the board verify after import, each in their own view of the tab "Data" (some affect the approved ballot design and must not be changed to repair an imported package); and the settings that administrators configure separately: the rules and required authorisations on the election event's tab "Signatures", certificate registrations, the self-test settings and the attack-detection thresholds |
| election execution data | the cast votes, the electoral log, the tally and results data, and the voting records |
| electoral log | the audit records of one election event. Each election event has its own electoral log; there is none for the tenant as a whole. What concerns the whole tenant, such as a change of the attack-detection settings or one of the tenant's accounts reaching the authentication failure threshold, is recorded in the electoral log of every election event of the tenant. The election board reads an election event's electoral log on the tab "Logs" of the election event and on the screen "Electoral Log" in the side menu; administrators do not read it |
| audit records | the records of the electoral log |
| operating logs | the logs of the servers and containers, which the operator reads to run the system. They are not audit records |
| the board's minutes | the election board's own record of the election event, kept outside the TOE in the form the electoral regulation sets ([Write the electoral regulation](02-organizer-duties.md#write-the-electoral-regulation)) |
| ballot box | where the TOE holds the cast votes of an election. During the execution phase this is what the Security Target calls the intermediate ballot box, which the TOE seals at the termination. At the counting it is the anonymised boards of the bulletin board that the trustees mix and decrypt |
| security-critical action | protected action (the election event's tab "Signatures", area "Protected actions") |
| required authorisations | the number in the field "Signatures needed" of the protected action's signing rule; an import of election data also needs the number of package approvals that the tenant requires |
| security token | the personal medium that holds a board member's signing certificate file |
| signing certificate | a board member's certificate that signs the TOE's signing requests |
| approval certificate | a board member's certificate that signs package approvals in the package tool |
| package approval | a board member's signature inside a signed configuration package, made with their approval certificate in the election organizer's package tool, outside the TOE, before the import |
| electoral phase | the preparation, execution, evaluation or post-processing phase ([The four phases](05-electoral-phases.md#the-four-phases)); the election event's tab "Security and self-tests" shows the current one |
| configuration package | the signed file of election data that the election board imports |
| configuration version | a published version of the election event's configuration, which the election board approves |
| approval event | an existing election event in the preparation phase in which the election board approves an import. Before the first import there is none, so the board creates an empty one, which holds no election data and never opens voting |
| approved baseline | the TOE's signed record of the election data and the access-control settings at the election board's approval of a configuration version. The self-tests compare against it. On screen: "board-approved configuration baseline" |
| lockdown period | the period the election board sets during which the configuration of the election event cannot change |
| maintenance mode | the state after a recoverable failure, on screen the state "Recovery required": voting is stopped until the election board resumes it after a passing self-test. A new election event also shows the state "Recovery required", with the reason "Startup self-tests are required", until voting opens for the first time; that is not a failure ([Maintenance mode at the start](12-modes-and-failures.md#maintenance-mode-at-the-start)) |
| unrecoverable failure | the state after a failure that cannot be recovered from: voting stays stopped and never resumes, and the election board terminates the election execution ([An unrecoverable failure](12-modes-and-failures.md#an-unrecoverable-failure)) |
| re-voting | voting again, where only the last vote counts; the field "Number of allowed votes" of an election sets how often a voter may vote |
| batch | one protocol board's part of a tally; a tally can have several |
| protocol board | the record of one batch, listed in the section "Election verification" of the results tally |
| bulletin board | the TOE's signed record of the electoral log, the ballot box seals and the trustees' protocol messages |
| configuration hash and Tally ID of a tally batch (the anchors) | the full value "Configuration hash" and the decimal value "Tally ID" beside that protocol board's button "Download verification data", in the section "Election verification" of the completed results tally on the election event's tab "Tally". This is the protocol-board identifier, not the tally-session ID. |
| trustee server | the part of the TOE (`braid`) that takes part in the ceremonies for one trustee on the protected server, with that trustee's keys file; one instance per trustee |

## Conventions

Screenshots illustrate real interfaces with synthetic test data. Sample
accounts, election names, identifiers, hashes and dates are not values to copy
into an election event. Follow the steps and required values in the text; a pictured
screen is not evidence that your own operation completed.

- Names in quotation marks, such as "Stop Voting" or "Run self-tests", are the
  exact texts of the admin portal in English: menu entries, tabs, buttons, fields,
  options, dialogs and messages.
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

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-10-27 | First edition for the evaluated configuration of v10.0.0. |
| 1.1 | 2026-11-02 | Numbered sections; consistent use of election event; all chapters revised; new screenshots. |
