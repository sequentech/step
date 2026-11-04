---
id: electoral_phases
title: Electoral Phases for the Election Board
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Every election event is in one of four electoral phases. The election event's
tab "Security and self-tests" shows the current phase and security state. The TOE sets the phase
from the election dates and from the election board's actions; nobody edits it
directly. The phase decides which functions the TOE allows.

## The four phases

| Electoral phase | Begins when | What happens | Chapters |
|---|---|---|---|
| Preparation | An empty approval event is created, or signed election data is imported | The election board approves and imports the election data; the administrators make the settings; the key ceremony is held; the board publishes and approves the configuration version and sets the lockdown period; the initialization report is generated. Voters cannot vote. | [Preparation Phase Procedures](06-preparation.md) |
| Execution | The election period start time is reached (scheduled start), or the election board selects the button "Start Voting" at or after that time | Voters cast their votes. The election board monitors the election event, interrupts and resumes it when needed. | [Execution Phase Procedures](07-execution.md) |
| Evaluation | The end of election time is reached (scheduled end), or the election board terminates the election execution with the button "Stop Voting" | The ballot boxes are sealed. The election board checks the audit records, counts the votes and determines the result. | [Evaluation and Post-Processing Procedures](08-evaluation-and-post-processing.md) |
| Post-processing | The election board selects the button "Start post-processing" on the election event's tab "Security and self-tests" after the result has been determined | The election board exports and keeps the election execution data, hands the verification data to the election organizer, and deletes the data when the regulation allows. | [Evaluation and Post-Processing Procedures](08-evaluation-and-post-processing.md) |

The phases follow each other in this order only. The TOE never returns an
election event to an earlier phase: once the election execution has been
terminated, voting cannot be opened again.

### Conditions for the start of the execution phase

The TOE opens voting, at the scheduled start or with the button "Start Voting", only if
all of these hold:

- the election period start time has been reached;
- the initialization report of each election has been generated (see
  [The initialization report](06-preparation.md#the-initialization-report));
- the current six-check self-test for the opening has succeeded;
- the ballot box is empty.

The start time must have arrived and the end time must not have passed.
Missing, failed or expired tests keep voting unavailable. A successful test
does not itself authorize a manual opening or recovery. A ballot box that is not
empty stops the start: the election event enters maintenance mode with the reason
"Ballot box not empty at the start of the election". After a failure or during a
board pause the TOE refuses a scheduled start: voting reopens only through the
election board
([Maintenance mode](12-modes-and-failures.md#maintenance-mode)).

## What each phase allows

| Function | Preparation | Execution | Evaluation | Post-processing |
|---|---|---|---|---|
| Import election data (signed configuration packages; the board approves them and a board member imports them) | Yes, with the required package approvals | No | No | No |
| Archive an election event imported with wrong data (election board, with the entry "Archive this Election Event" of the election event's menu) | Yes | No | No | No |
| Change the settings and the signing rules (administrators) | Yes | No | No | No |
| Publish a configuration version (election board, with the buttons "Generate Publication" and "Publish Changes" on the election event's tab "Publish") | Yes, approved by the board (the protected action "Approve a configuration version") | No | No | No |
| Enter or change election data in any other way | No | No | No | No |
| Voters log in (subject to account and audit-storage restrictions) | Yes | Yes | Yes | Yes |
| Voters vote and check their cast ballots in the ballot locator | No | Yes | No | No |
| Interrupt with the button "Pause Voting", resume with the button "Start Voting", then the entry "Start Online Voting" | No | Yes; the resumption with the required authorisations | No | No |
| Terminate the election execution with the button "Stop Voting" | No | Yes, with the required authorisations | No | No |
| Request the six security self-tests | Yes | Yes | Yes | Yes |
| Read the audit records (the election event's tab "Logs" or the screen "Electoral Log" in the side menu, election board only) | Yes | Yes | Yes | Yes |
| Count the votes and determine the result | No | No | Yes, with the required authorisations | No |
| Export election execution data | No | No | No | Yes, with the required authorisations |
| Read the cast votes as election board | No | No | No | Yes, in the export, with the required authorisations |
| Read the voters' register as election board | Yes | Yes | Yes | Yes, with the required authorisations |
| Delete election execution data | No | No | No | Yes, after the export, with the required authorisations |

The TOE checks the phase and required board authorizations on the server,
including requests made outside the visible portal controls.

The initialization report is the one count that the TOE allows before the
execution phase: it counts the empty ballot box to show that it holds no
votes.

## Security-critical actions

The Security Target requires the prescribed number of election board members
to authorise security-critical actions (see
[Authorising a security-critical action](04-secure-use.md#authorising-a-security-critical-action)).
For an import, the board first approves the signed configuration package and
then authorises the exact uploaded file through the protected action "Import
election data" in an existing election event in the preparation phase. Create an
empty approval event for the first import
([Prepare the first import's approval event](06-preparation.md#prepare-the-first-imports-approval-event)).
The TOE checks both the package approvals and the fresh authorisations.
The TOE lists the other protected actions on the election event's tab
"Signatures", area "Protected actions", with the number of authorisations each
needs in the column "Signatures needed".

The following table identifies the protected operations and permitted phases.

| Security-critical action | How it is started | Phase |
|---|---|---|
| Import of the ballot (candidate list and its display data) | A board member selects the entry "Import Election Event" with a signed configuration package; with valid package approvals and a completed signing request for the protected action "Import election data" | Preparation |
| Import of the voters' register | A board member selects the entry "Import Election Event" with a signed configuration package; with valid package approvals and a completed signing request for the protected action "Import election data" | Preparation |
| Import of the set of election rules | A board member selects the entry "Import Election Event" with a signed configuration package; with valid package approvals and a completed signing request for the protected action "Import election data" | Preparation |
| Import of the election dates | A board member selects the entry "Import Election Event" with a signed configuration package; with valid package approvals and a completed signing request for the protected action "Import election data" | Preparation |
| Termination of the election execution | The button "Stop Voting" (protected action "Close voting") | Execution |
| Resumption of the election execution | The button "Start Voting", then the entry "Start Online Voting", on the paused election (protected action "Open voting") | Execution |
| Start of counting votes with determination of the election result | The buttons "Start Tally Ceremony" and "Start Tally" on the election event's tab "Tally" | Evaluation |
| Export of election execution data | The button "Export" on the election event's tab "Data" | Post-processing |
| Read of the voters' register by the election board | The election event's tab "Voters" | Post-processing |
| Deletion of election execution data | The entry "Remove this Election Event" of the election event's menu | Post-processing |

The board's authorization is bound to the requested operation, election event or election, and
input. Changing the input, phase or applicable data invalidates the authorization;
a signature from an earlier request does not approve a retry with different data.
Trustee custody and report signatures remain separate approvals.

Delete only in the post-processing phase after a completed, valid export of the
current election execution data and the required board approvals. A failed or
obsolete export cannot permit deletion. Archive an incorrectly imported election
event during the preparation phase instead of deleting its evidence (see
[Importing the election data](06-preparation.md#importing-the-election-data)).

The evaluated configuration also protects these actions, which the TOE
offers in its catalogue on the same tab:

| Protected action | What it covers |
|---|---|
| "Approve a configuration version" | Publication of a new configuration version of the election event (see [Approving the configuration](06-preparation.md#approving-the-configuration)) |
| "Initialize voting" | Generation of the initialization report of an election |
| "Open voting" | Start of voting with the button "Start Voting", and its resumption |
| "Generate other election reports" | Release of the signed initialization report and of other election reports |
| "Confirm custody (key ceremony)" | Each trustee's confirmation of custody of their keys at the key ceremony |
| "Authorize tally custody (tally)" | Each trustee's authorisation of the counting |

Rules for the signing requests and package approvals:

- Except for the two trustee custody rows, the administrators set the column "Signatures
  needed" in the preparation phase, as the
  electoral regulation fixes it. Mandatory board actions cannot use an absent or disabled rule to bypass
  authorization. Check the configured number and available signers before proceeding.
  For the imports they set the
  tenant's number of package approvals before a board member creates the first
  election event, and never
  change it afterwards; while it is not set, the TOE refuses every import.
  For the two trustee custody rows, the switch "Trustees sign this step" is on: each
  trustee signs their own step, with no configurable signature count.
- Each authorisation must come from a different board member. The TOE accepts
  each signing certificate only once per request.
- Any board member can abort a security-critical action before it runs, with
  the button "Cancel request" ([Aborting](04-secure-use.md#aborting)). A request
  also ends if what it would sign changes, or when it expires. Cancel an import's signing request before selecting the button "Continue operation"
  to stop that pending operation; tell the election organizer as well.
- Every step (request created, signed, refused, cancelled, expired, completed,
  action performed) is recorded in the electoral log.

:::warning
Check on the election event's tab "Signatures" before the preparation phase ends
that every protected action except the two trustee custody actions has the
column "Signatures needed" set to the number in the electoral regulation, and that the board has
at least that many members who can sign. For the imports, have the
administrators show the tenant's number of package approvals. For the trustee
custody rows, check that the switch "Trustees sign this step" is on. If the column
"Signatures needed" of a protected action shows "Off", or a lower number than the regulation fixes, stop and have the administrators correct it before
the execution phase. Only the rows "Generate election returns", "Transmit results"
and "Approve a voter manually" may show "Off": the evaluated configuration does not
use them
([Protected actions to switch on](../04-administrator_handbook/06-signing_rules.md#protected-actions-to-switch-on)).
:::

## Confirmation notice

Before the execution phase is terminated, the TOE shows a confirmation notice:
the dialog "Confirm Action", which says that the ballot boxes are then sealed, so
that no ballot can be added, changed or deleted, and that voting cannot start
again. Each authorising board member then authenticates again at the
external authentication server before their authorisation is accepted (see
[Confirming an action by authenticating again](04-secure-use.md#confirming-an-action-by-authenticating-again)).
The termination runs only after the board member has confirmed and the
required authorisations are complete.
