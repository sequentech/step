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
     decision; correlate `MaintenanceModeEntered`, `MaintenanceModeLeft` and
     `ElectoralPhaseChanged` with the security state and operational record;
   - the required six-check runs completed successfully for the relevant
     election state, and every failed, missing, delayed or expired run was
     investigated; check the separate electoral-log audits as well;
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

Counting is a protected board operation. The TOE checks the evaluation phase,
the required board authorization, trustee custody and the tally prerequisites.
When **Before counting** is selected in the self-test settings, that current
purpose-specific suite must also pass. Trustee custody and later report
signatures do not replace the board's authorization to start counting.

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
4. Complete the displayed signing request with the required distinct board
   members. The original requester selects **Continue operation** when the
   request is complete. If a before-counting test was requested instead, wait
   for all six results, then request fresh authorization.
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

1. In **Security and self-tests**, confirm **Evaluation** and that every
   election has a successfully completed results tally.
2. Select **Start post-processing**. Read the irreversible phase-change notice.
3. If sent to the external authentication server, authenticate again, return
   and confirm the phase change.
4. Check that the state is **Post-processing**. Further counting and reopening
   voting are refused.

After an **Unrecoverable failure**, first terminate execution under the board
procedure. The same control permits investigation export from evaluation
without a completed result. This exception is recorded; it does not turn a
failed election into a valid result or allow it to resume.

## Reading the cast votes and the voters' register

In post-processing the board may read cast votes and the voters' register
within its assigned permissions. Reading the register in this phase requires
the protected request's distinct board approvals. Review the request's scope,
complete its signatures and let the original requester select **Continue
operation**. The approved reader has a five-minute session for register counts,
pages and individual records in that same event and state. A different reader,
changed election data or expired session requires new approval. Revealing
secret fields and exporting data remain separately protected operations.
Read only what the electoral regulation permits and record the reason.

## Exporting the election execution data

The export keeps the election execution data (the cast votes, the electoral
log, the tally and results data and the voting records) for the time that the
electoral regulation fixes. Perform it only in the post-processing phase.
The TOE requires the execution-export permission, post-processing and the
required board signatures. The approval binds the selected export options and
current election data.

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
4. Select **Export**, review the signing request and obtain the required
   distinct board approvals. The original requester selects **Continue
   operation** to continue the same export with the same options.
5. The TOE generates the password of the file; nobody chooses it. The admin
   portal shows it once, under "Password to decrypt the file:". Write the
   password into the board's protected record at once. Never store it with
   the file.
6. The export runs as the task "Export Election Event" on the Tasks tab. When
   it is complete, download the file.
7. The archive contains `execution-integrity/manifest.json`, a signature and
   its public key. The manifest lists SHA-256 hashes of the exported entries.
   It also covers `execution-security-evidence.json`: the event's security
   settings, applicable attack detections, pending security notices and its
   pseudonymous authentication activity. Keep this file with the archive;
   an audit-log export alone does not include all this evidence.
   Keep the `ExecutionDataExported` record's archive hash, manifest hash and
   public key separately with the board's minutes. The recipient checks the
   archive and signed manifest against those independently retained values;
   a public key taken only from the same untrusted archive is insufficient.
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
   If new security evidence has arrived since that export, obtain fresh board
   approvals and export again before requesting deletion.
2. In the election event's menu, select "Remove this Election Event" and
   confirm "Are you sure to delete this item?".
3. Review the protected deletion request. Obtain the required distinct board
   approvals; the original requester selects **Continue operation**.
4. The TOE verifies the completed export receipt against the current data and
   lifecycle state. A missing, failed or obsolete export blocks deletion.
   Preserve the refusal and investigate; never remove seals or alter receipts
   to make deletion pass.
5. Check that the election event no longer appears in the list of election
   events, and record the deletion in the board's minutes.

"Archive this Election Event" keeps all data and audit records of the
election event and is not a substitute for deletion. The board uses it in the
preparation phase for an election event imported with wrong data (see
[Importing the election data](06-preparation.md#importing-the-election-data)).
An archived election event never opens voting and is never unarchived.
