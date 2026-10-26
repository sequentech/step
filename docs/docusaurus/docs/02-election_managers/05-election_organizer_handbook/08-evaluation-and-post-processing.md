---
id: evaluation_post_processing
title: Evaluation and Post-Processing Procedures
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

After the termination of the election execution the election event is in the
evaluation phase. The election board checks the audit records, counts the votes
and determines the result. It then moves the election event to the
post-processing phase, where it exports, publishes and finally deletes the
election execution data.

## Checking the audit records

At the beginning of the evaluation phase, before the votes are counted, the
election board (or a person it appoints) checks the audit records of the whole
election.

1. On the election event's Logs tab, select "Audit". The dialog "Audit
   electoral log" says: "The audit checks every log entry against its Merkle
   log and verifies the published checkpoints. The result is recorded as a
   task." Select "Audit".
2. On the Tasks tab, wait for the task "Audit Electoral Log". It must end
   successfully. A failed task lists its findings in its logs.
3. Read the records on the Logs tab for the whole preparation and execution
   phases. Check at least:
   - every opening, interruption, resumption and termination
     (`ElectionVotingPeriodOpen`, `ElectionVotingPeriodPause`,
     `ElectionVotingPeriodClose` and their `ElectionEvent…` forms) matches a
     decision of the board or the election dates;
   - each operator-reported interruption has a recorded cause and board
     decision; automatic maintenance-mode events are not implemented, so
     their absence is not evidence that no failure occurred;
   - each scheduled **Audit Electoral Log** task completed without findings,
     and any missing or delayed run was investigated; this is not a complete
     ST self-test record;
   - every ballot box has a `BallotBoxSealed` record, and there is no
     `BallotBoxSealFailed`;
   - there is no `AuthenticationFailureThresholdReached` or
     `AuditStorageThresholdExceeded` record without an explanation, and the
     Monitoring tab's "Attack detections" are explained;
   - every security-critical action was authorised by the required number of
     different board members (the `Signing…` records), and every refused,
     cancelled or failed signing request (`SigningSignatureRefused`,
     `SigningRequestCancelled`, `SigningCertificateOpenFailed`) is explained;
   - no change of signing rules, permissions, issuers or certificate checks
     (`SigningRuleChanged`, `SigningPermissionChanged`, `SigningIssuerChanged`,
     `SigningChecksChanged`), and no import of election data
     (`ConfigurationPackageImported`), happened after the preparation phase;
   - the failed logins in the `KeycloakUserEvent` records show no pattern of
     attack.
4. To keep the records with the board's minutes, export them with "Export in
   CSV" or "Export in PDF" (permissions "Export Logs" and "Edit Reports").
5. Write the result of the check in the board's minutes.

:::warning
If the audit task fails or a record shows an event that the board cannot
explain, do not count the votes yet. Treat it as a security-relevant event and
decide under the electoral regulation whether the election can be evaluated.
The export of the records contains personal data of voters and board members:
keep it confidential.
:::

## Counting the votes

Counting the votes and determining the election result is a security-critical
action under the Security Target. The current results-tally path checks the
required trustee custody and tally prerequisites, but has no separate
board-signing gate for starting the count. This implementation limitation must
be resolved before claiming the required board-quorum enforcement; later report
signatures do not authorise the earlier count.

Before you start, check that:

- the election event is in the evaluation phase;
- on the Dashboard, every ballot box of the election is "Sealed" ("An
  election can be tallied once every ballot box is sealed and its seal record
  is published.");
- the audit records have been checked;
- both trustees are present with their own encrypted media and access to the
  controlled offline workstation (which they may use in turn), and their
  cluster trustee instances are running (the
  operators confirm it).

Steps:

1. On the election event's Tally tab, select "Start Tally Ceremony".
2. Under "Key Ceremony", select the key ceremony of the election event. Under
   "Elections to Tally", select the elections to count.
3. Select "Start Tally Ceremony" and confirm "Are you sure you want to start a
   ceremony?" ("This tally ceremony asks trustees to submit public custody
   proofs. Keep their runners active with their local protocol state.").
4. Record the board’s required decision under the electoral regulation. The
   current UI does not request a separate board signature for starting this
   results tally; this record does not remedy the implementation limitation
   described above.
5. Each trustee authorises the counting with their keys, as
   [Authorising the counting](09-trustee-procedures.md#authorising-the-counting)
   describes. The section "Trustees" ("Custody confirmation status") shows how
   many trustees have confirmed custody and how many are needed.
6. When enough trustees have confirmed, select "Start Tally" and confirm "The
   required trustees have confirmed custody. Start the tally with their
   runners active?" with "Start Tally".
7. The TOE checks every ballot box against its seal, then the trustees mix and
   decrypt the votes and the TOE determines the result. "Elections Tally
   Progress" shows the progress.
8. When the tally is complete, the result appears under "Results &
   Participation".

:::warning
The tally ceremony runs only as a manual ceremony and starts only with the
trustees' authorisations. Stop, and treat it as a security-relevant event, if:

- the wizard offers "Start Tally" before any trustee has confirmed custody, or
  shows "Select Start Tally to run tally process and display results, or Close
  to cancel." (an automatic ceremony);
- a trustee is shown as having confirmed custody who did not do so;
- the TOE reports that a ballot box does not match its seal.

In each case select "Cancel Tally" if the tally has not started, and do not
publish any result.
:::

## Checking and publishing the result

1. Check the result under "Results & Participation" for each election: the
   number of eligible voters, the number of votes, the votes counted, valid
   and invalid, and the votes of each candidate. The number of votes counted
   must agree with the ballot boxes on the Dashboard ("Counted").
2. Generate the reports the electoral regulation requires. Where "Generate
   other election reports" needs signatures, the required board members sign
   each report.
3. The election board determines the result and records it, with the signed
   reports, in its minutes. The election organizer announces it through its
   own channels. The evaluated configuration has no results website: the TOE
   refuses to import a package that enables it, and no role holds "Edit
   Results Publication" (`publish-results-write`), so "Publish to results
   website" is never available. If it is offered, do not use it, and treat it as a
   security-relevant event.

After the tally, run "Audit" on the Logs tab once more, as in
[Checking the audit records](#checking-the-audit-records), and check that the
new "Audit Electoral Log" task on the Tasks tab ends successfully. Record the
result in the board's minutes.

## Starting post-processing

When the election result has been determined, a board member selects "Start
post-processing" on the election event's Dashboard. The election event moves
to the post-processing phase: the Dashboard shows "Electoral phase" Post-processing. From now on no
counting is possible, and the export and the deletion of election execution
data become possible.

After an unrecoverable failure (see
[An unrecoverable failure](12-modes-and-failures.md#an-unrecoverable-failure))
the election board may select "Start post-processing" without an election
result. The Dashboard then shows that no result was determined, and the
election execution data can be exported for the investigation.

## Reading the cast votes and the voters' register

In the post-processing phase the election board can read the cast votes and the
voters' register, for example to answer a challenge of the election. The
board's read of the voters' register is a security-critical action under the
Security Target’s post-processing condition, but the current read path checks
permission without a post-processing board-signature gate. This is an
implementation limitation for that condition; it does not mean that every
ordinary board read requires signatures (the Security Target also permits
general board access to the register).
Read only what the electoral regulation allows, and record each read and its
reason; the minutes are not evidence of TOE-enforced quorum.

## Exporting the election execution data

The export keeps the election execution data (the cast votes, the electoral
log, the tally and results data and the voting records) for the time that the
electoral regulation fixes. Perform it only in the post-processing phase.
The current full-event export path has no corresponding phase or board-signing
gate; its general election-event read permission must not be interpreted as
proof of the required board authorisations. This is an implementation limitation,
not permission to export outside the prescribed procedure.

1. On the election event's Data tab, select "Export". The drawer "Export
   Election Event" opens.
2. Tick "Encrypt with Password". Always.
3. Tick the parts the electoral regulation requires to be kept, at least
   "Activity Logs", "Bulletin Board" and "Tally". Tick "Include Voters" only if
   the regulation requires the voters' register to be kept with the data. With
   "Encrypt with Password", and if the member who starts the export holds
   "Reveal Secret Voter Fields", the export includes the voters' secret data
   in decrypted form: the secret fields of the voters' register with "Include
   Voters", and the voters' secret documents among the stored files with "S3
   Files".
4. Obtain the board’s decision under the electoral regulation before selecting
   "Export". The current export does not request a board-signing quorum; do not
   record it as a TOE-enforced protected action.
5. The TOE generates the password of the file; nobody chooses it. The admin
   portal shows it once, under "Password to decrypt the file:". Write the
   password into the board's protected record at once. Never store it with
   the file.
6. The export runs as the task "Export Election Event" on the Tasks tab. When
   it is complete, download the file.
7. The TOE signs the export with a key that it generates for this export and
   destroys right after signing; the file carries the signature and the
   public key. It records that the election execution data has been exported
   and writes the fingerprint of the public key into the electoral-log record
   of the export. Copy the fingerprint into the board's minutes and hand it
   over with the file: whoever receives the file checks the signature with
   the public key and compares the key's fingerprint with the minutes.
8. Keep at least two copies of the file on separate protected media, each
   under the control of at least two board members, and the password apart
   from them.

:::warning
The export contains everything needed to reconstruct the election, including
confidential data of the election event. Whoever has the file and its
password has that data. Never send the file or the password by email or chat,
never keep them together, and destroy the copies when the electoral regulation
says so.
:::

## Downloading the verification data

In the post-processing phase, after voting has closed for every election in
the tally, open the completed **results tally** from the Tally tab. In
**Election verification**, each listed protocol board represents one batch.
For every batch, record the full **Configuration hash** and **Tally ID** from
that board's section, then select its **Download verification data** button.
The control does not export initialization reports or key-ceremony-only data.

Each `verification-<protocol-board UUID>.json` file contains the public protocol
messages for that batch's key generation, mixing and decryption. Obtain its
trusted anchors separately from the platform's stored configuration and board
record; do not choose them from the downloaded transcript. The displayed
**Tally ID** is that protocol-board UUID written as a complete unsigned decimal
number, not the tally-session ID. Keep every file paired with its own anchors
in the board's minutes and hand all batches to the election organizer.

Verify every expected batch and publish every file with its matching anchors,
including a second, independent publication of the anchors, as
[Verifying the Result with the Election Verifier](10-election-verifier.md)
describes. A successful check of one batch does not verify the complete tally.

## Deleting the election execution data

When the electoral regulation's retention period has ended, the election board
deletes the election execution data (phase: post-processing; permission
"Delete Election Event", `election-event-delete`, which only the election
board role holds):

:::warning
Deleting the election event also deletes its audit records in the TOE; only
the exported copies remain. Delete only after the export (see
[Exporting the election execution data](#exporting-the-election-execution-data))
and after the lockdown period has ended: the TOE refuses the deletion while
the election event is locked down.
:::

1. Check that the data has been exported and that the copies are readable.
2. In the election event's menu, select "Remove this Election Event" and
   confirm "Are you sure to delete this item?".
3. Record the required board decision. The current deletion path does not
   request board signatures or verify that an export was completed; these
   missing prerequisites are implementation limitations.
4. The current implementation refuses deletion of an event with sealed ballot
   boxes, including after export. Do not bypass that refusal or remove seals
   to complete retention work. Report the blocked deletion to the operators.
5. Check that the election event no longer appears in the list of election
   events, and record the deletion in the board's minutes.

"Archive this Election Event" keeps all data and audit records of the
election event and is not a substitute for deletion. The board uses it in the
preparation phase for an election event imported with wrong data (see
[Importing the election data](06-preparation.md#importing-the-election-data)).
An archived election event never opens voting and is never unarchived.
