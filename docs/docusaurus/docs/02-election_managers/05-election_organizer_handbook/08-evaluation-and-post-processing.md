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
election event. To check the audit records:

1. On the election event's tab "Logs", select the button "Audit". The dialog "Audit
   electoral log" says that the audit checks every log entry and the published
   checkpoints, and that the result is recorded as a task. Select its button
   "Audit".
2. On the election event's tab "Tasks", wait for the task "Audit Electoral Log". It
   must end successfully. A failed task lists its findings in its logs.
3. Read the records on the tab "Logs", or on the screen "Electoral Log" in the side
   menu, for the whole preparation and execution phases. Check at least:
   - every opening, interruption, resumption and termination
     (`ElectionVotingPeriodOpen`, `ElectionVotingPeriodPause`,
     `ElectionVotingPeriodClose` and their `ElectionEvent…` forms) matches a
     decision of the board or the election dates;
   - each interruption the operators reported has its cause and the board's
     decision in the board's minutes; compare the records `MaintenanceModeEntered`,
     `MaintenanceModeLeft` and `ElectoralPhaseChanged` (of the kind
     `LifecycleSecurityEvent`, with the name at the start of their description)
     with the board's minutes;
   - the required self-test runs completed successfully for the relevant
     election event state, and every failed, missing, delayed or expired run was
     investigated; check the separate electoral-log audits as well;
   - every ballot box has a `BallotBoxSealed` record, and there is no
     `BallotBoxSealFailed`;
   - there is no record `PotentialAttackDetected`, `AuditStorageThresholdExceeded`
     or `AuditStorageObservationUnavailable` without an explanation, and the
     dashboard "Attack detections" on the election event's tab "Dashboard" shows
     nothing unexplained;
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
4. To keep the records with the board's minutes, export them with the button
   "Export" on the tab "Logs", which asks for the period, the time zone and the
   format, CSV or PDF. This needs the permission "Export Logs" (`logs-export`).
5. Write the result of the check in the board's minutes.

:::warning
If the audit task fails or a record shows an event that the board cannot
explain, do not count the votes yet. Treat it as a security-relevant event and
decide under the electoral regulation whether the election event can be evaluated.
The export of the records contains personal data of voters and board members:
keep it confidential.
:::

## Counting the votes

Counting is a protected board operation. The TOE checks the evaluation phase,
the required board authorization, trustee custody and the tally prerequisites.
When the additional condition "Before counting" is selected in the self-test
settings, a run with that purpose must also have passed at most five minutes before
the board's request, and must still be the latest finished run when counting
executes. Trustee custody and later report
signatures do not replace the board's authorization to start counting.

Before you start, check that:

- the election event is in the evaluation phase;
- on the tab "Dashboard" of each election to count, the status of every ballot box
  is "Sealed": an election can be counted only once every ballot box is sealed and
  its seal record is published;
- the audit records have been checked;
- both trustees are present with their own encrypted media and access to the
  controlled offline workstation (which they may use in turn), and their
  instances of the trustee server are running (the operators confirm it).

To count the votes:

1. On the election event's tab "Tally", select the button "Start Tally Ceremony".
2. In the section "Key Ceremony", select the key ceremony of the election event. In
   the section "Elections to Tally", select the elections to count.
3. Select the button "Start Tally Ceremony". Confirm the dialog with its button "Ok":
   it says that the ceremony asks the trustees to submit public custody proofs, and
   that their instances of the trustee server must stay active.
4. If the TOE answers "The configured before-counting self-test has been
   requested. Wait for all six checks to pass, then authorize counting again.",
   wait for all six results and select the button "Start Tally Ceremony" again.
5. Complete the displayed signing request with the required distinct board
   members, before the request expires. The original requester selects
   **Continue operation** when the request is complete. If another self-test
   finished in the meantime, the TOE gives the answer of step 4 and keeps the
   signatures: act as in step 4. If a test failed, the TOE cancelled the request:
   after the recovery, start again from step 3.
6. Each trustee authorises the counting with their keys, as
   [Authorising the counting](09-trustee-procedures.md#authorising-the-counting)
   describes. The section "Trustees" shows how many trustees have confirmed custody
   and how many are needed.
7. When enough trustees have confirmed, select the button "Start Tally", and confirm
   the dialog with its button "Start Tally".
8. The TOE checks every ballot box against its seal, then the trustees mix and
   decrypt the votes and the TOE determines the result. The section "Elections
   Tally Progress" shows the progress.
9. When the tally is complete, the result appears in the section "Results &
   Participation".

:::warning
The tally ceremony runs only as a manual ceremony and starts only with the
trustees' authorisations. Stop, and treat it as a security-relevant event, if:

- the wizard offers the button "Start Tally" before any trustee has confirmed
  custody, or shows the message "Select Start Tally to run tally process and display
  results, or Close to cancel." (an automatic ceremony);
- a trustee is shown as having confirmed custody who did not do so;
- the TOE reports that a ballot box does not match its seal.

In each case select the button "Cancel Tally" if the tally has not started, and do not
publish any result.
:::

## Checking and publishing the result

To check and publish the result:

1. Check the result in the section "Results & Participation" for each election: the
   number of eligible voters, the number of votes, the votes counted, valid
   and invalid, and the votes of each candidate. The number of votes counted
   must agree with the column "Counted" of the table of ballot boxes (the switch
   "Show ballot boxes" on the tab "Dashboard").
2. Open **Actions** for the election result. A report marked **awaiting
   signatures** is held until its own approval is complete. Select **Open the
   signing request**, review the document and configuration revision, and have
   the required distinct board members sign with their personal certificates.
   Repeat for each required area report; approving the election report does
   not approve its area reports.
3. Reopen **Actions** and select the entry "Export in PDF format - '…' results" for
   the report you approved. Check the downloaded PDF's signatures, signer identities,
   configuration revision and totals. If it still says **awaiting signatures**,
   check the request's approvals and any refused certificate check. If the
   request has completed but the PDF is unavailable, ask the operators to
   investigate before publishing it.
4. Generate any additional reports the electoral regulation requires. Where
   the protected action "Generate other election reports" needs signatures, the
   required board members sign each report. The evaluated configuration keeps each
   report's setting "Encryption Policy" at "Unencrypted": no requirement needs
   encrypted reports, and reading one needs a command on a staff computer.
5. The election board determines the result and records it, with the signed
   reports, in the board's minutes. The election organizer announces it through its
   own channels. The evaluated configuration has no results website: the TOE
   refuses to import a package that enables it, and no role holds the permission
   "Edit Results Publication" (`publish-results-write`), so the section "Publish to
   results website" is never available. If it is offered, do not use it, and treat it as a
   security-relevant event.

![Completed result with five voters and separate downloadable election and area PDFs](./assets/results-released-reports.png)

*Illustration: a synthetic election event. The election and area PDFs became
available after their separate approvals; use your election's expected totals
and authorised signers when checking your own reports.*

After the tally, select the button "Audit" on the tab "Logs" once more, as in
[Checking the audit records](#checking-the-audit-records), and check that the
new task "Audit Electoral Log" on the tab "Tasks" ends successfully. Record the
result in the board's minutes.

## Starting post-processing

To start the post-processing phase:

1. On the election event's tab "Security and self-tests", confirm that the phase is
   "Evaluation" and that every election has a successfully completed results tally.
2. Select the button "Start post-processing". Read the notice in the dialog, which
   says that this phase change cannot be undone.
3. If you are sent to the external authentication server, authenticate again, return
   and select the button "Confirm phase change".
4. Check that the phase is "Post-processing". Further counting and reopening
   voting are refused.

After an unrecoverable failure ([An unrecoverable failure](12-modes-and-failures.md#an-unrecoverable-failure)),
first terminate the election execution. The button "Start post-processing" then
works in the evaluation phase without a completed result, so that the board can
export the data for the investigation. The electoral log records this exception; it
does not turn a failed election event into a valid result or allow it to resume.

## Reading the cast votes and the voters' register

In the post-processing phase the board may read cast votes and the voters' register
within its assigned permissions. Reading the register in this phase requires
the protected request's distinct board approvals. Review the request's scope,
complete its signatures and let the original requester select **Continue
operation**. The approved reader has a five-minute session for register counts,
pages and individual records in that same election event and state. A different reader,
changed election data or expired session requires new approval. Revealing
secret fields and exporting data remain separately protected operations.
Read only what the electoral regulation permits, and write the reason in the
board's minutes.

## Exporting the election execution data

The export keeps the election execution data (the cast votes, the electoral
log, the tally and results data and the voting records) for the time that the
electoral regulation fixes. Perform it only in the post-processing phase.
The TOE requires the permission "Export election execution data"
(`election-execution-export`), the post-processing phase and the required board
signatures. The approval binds the selected export options and
current election data. To export the election execution data:

1. On the election event's tab "Data", select the button "Export". The drawer "Export
   Election Event" opens.
2. Tick "Encrypt with Password". Always.
3. Tick the parts the electoral regulation requires to be kept, at least
   "Activity Logs", "Bulletin Board" and "Tally". Tick "Include Voters" only if
   the regulation requires the voters' register to be kept with the data. With
   the option "Encrypt with Password", and if the member who starts the export holds the
   permission "Reveal Secret Voter Fields", the export includes the voters' secret data
   in decrypted form: the secret fields of the voters' register with the option
   "Include Voters", and the voters' secret documents among the stored files with
   the option "S3 Files".
4. Select the button "Export", review the signing request and obtain the required
   distinct board approvals. The original requester selects **Continue
   operation** to continue the same export with the same options.
5. The TOE generates the password of the file; nobody chooses it. The admin
   portal shows it once, under the label "Password to decrypt the file:". Write the
   password down at once, and keep it under the control of at least two board
   members. Never store it with the file.
6. The export runs as the task "Export Election Event" on the election event's tab
   "Tasks". When it is complete, download the file.
7. The archive contains `execution-integrity/manifest.json`, a signature and
   its public key. The manifest lists SHA-256 hashes of the exported entries.
   It also covers `execution-security-evidence.json`: the election event's security
   settings, full self-test history with each check's result and start/completion
   times, the approved baseline, applicable attack detections, pending security
   notices and its
   pseudonymous authentication activity. Keep this file with the archive;
   an export of the electoral log alone does not include all this evidence.
   Keep the `ExecutionDataExported` record's archive hash, manifest hash and
   public key separately with the board's minutes. The recipient checks the
   archive and signed manifest against those independently retained values;
   a public key taken only from the same untrusted archive is insufficient.
8. Keep at least two copies of the file on separate protected media, each
   under the control of at least two board members, and the password apart
   from them.

:::warning
The export contains everything needed to reconstruct the election event, including
its confidential data. Whoever has the file and its
password has that data. Never send the file or the password by email or chat,
never keep them together, and destroy the copies when the electoral regulation
says so.
:::

## Downloading the verification data

After voting has closed for every election in the tally, open the completed
results tally from the election event's tab "Tally". Check its verification data
before publishing the result. In the section "Election verification", each listed
protocol board represents one batch. For every batch, write the full values
"Configuration hash" and "Tally ID" of that board's section in the board's minutes,
then select its button "Download verification data". The button does not export
initialization reports or key-ceremony-only data.

Each `verification-<protocol-board UUID>.json` file contains the public protocol
messages for that batch's key generation, mixing and decryption. Its trusted
anchors are the two values written in the board's minutes; do not take them from
the downloaded file. The displayed value "Tally ID" is that protocol-board UUID written as a complete unsigned decimal
number, not the tally-session ID. Keep every file paired with its own anchors
in the board's minutes and hand all batches to the election organizer.

Verify every expected batch and publish every file with its matching anchors,
including a second, independent publication of the anchors, as
[Verifying the Result with the Election Verifier](10-election-verifier.md)
describes. A successful check of one batch does not verify the complete tally.

## Deleting the election execution data

When the electoral regulation's retention period has ended, the election board
deletes the election execution data, in the post-processing phase. This needs the
permission "Delete Election Event" (`election-event-delete`), which only the role
`election-board` holds.

:::warning
Deleting the election event removes its operational data and electoral log.
Signed protocol evidence and restricted records of approvals, exports,
security notices and configuration revisions remain for verification and
replay prevention. Deletion does not erase those records.

Delete only after the export (see
[Exporting the election execution data](#exporting-the-election-execution-data))
and after the lockdown period has ended.
:::

To delete the election execution data:

1. Check that the data has been exported and that the copies are readable.
   If new security evidence has arrived since that export, obtain fresh board
   approvals and export again before requesting deletion.
2. In the election event's menu, select the entry "Remove this Election Event" and
   confirm the question "Are you sure to delete this item?".
3. Review the protected deletion request. Obtain the required distinct board
   approvals; the original requester selects **Continue operation**.
4. The TOE verifies the completed export receipt against the current data and
   lifecycle state. A missing, failed or obsolete export blocks deletion.
   Preserve the refusal and investigate; never remove seals or alter receipts
   to make deletion pass.
5. Check that the election event no longer appears in the list of election
   events, and record the deletion in the board's minutes.

The menu entry "Archive this Election Event" keeps all data and audit records of the
election event and is not a substitute for deletion. The board uses it in the
preparation phase for an election event imported with wrong data (see
[Importing the election data](06-preparation.md#importing-the-election-data)).
An archived election event never opens voting, and nothing sets it back.
