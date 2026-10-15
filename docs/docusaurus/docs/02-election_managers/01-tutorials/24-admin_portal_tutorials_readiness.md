---
id: admin_portal_tutorials_readiness
title: Election Readiness Rehearsal and Lockdown
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Use separate election events for the end-to-end readiness test (ERT), final
testing and locking (FTL), and official voting. Keep the ERT and FTL events as
closed, sealed, archived evidence. The official event remains unopened until
its approved voting start. This procedure requires the election authority's
approval of separate events as the re-zero method, operator permissions,
witnesses, credential isolation, key custody and the test schedule. A readiness
report records technical checks at one time; it does not record that approval.

## Roles and preparation

Assign an election manager, an operator with `readiness-check`, an auditor who
witnesses and signs the minutes, and the SBEI members who authorize opening and
closing. Confirm the approved lockdown end is after voting closes in every Post
and before tally. Record an approved exception if the FTL lead time is shorter
than the existing seven-day warning.

In the source configuration, choose **Required SHA-256** for voter-file integrity
and **Seal at close** before producing the three approved packages. The existing
lifecycle preset includes both policies. Policy fields present in the JSON
participate in the signed ballot-design digest, so changing them after admission
can make publication verification fail. If an admitted package needs a policy
correction, obtain corrected, signed and approved packages at higher revisions
instead of editing the admitted signed content.

Prepare three approved configuration packages with distinct, increasing revisions
and identical approved content and verified approver identities. Their manifest
hashes may differ. Verify that the configuration producer can issue these
packages before beginning; ordinary import freshness and duplicate protections
still apply. Any subsequent official correction must exceed the highest revision
already imported, including revisions used for tests. Do not edit signed package
contents to label an event as ERT or FTL.

Keep a mapping of each event's identifier and its canonical signed Post names.
Receipt keys are trimmed area names matching `manifest.content.ballot_designs[].area`;
runtime area UUIDs differ between events.
Follow the [configuration import guide](../../07-developers/10-tutorials/06-import-election-event.md),
[publishing tutorial](./12-admin_portal_tutorials_publish-election.md),
[scheduling tutorial](./13-admin_portal_tutorials_scheduled-events.md) and
[seal runbook](./22-admin_portal_tutorials_ballot-box-seal-runbook.md).
Use the supported ceremony export/import and trustee custody procedure for any
shared-key rehearsal; retain its transfer evidence and verify it with the
custody team. A separate ceremony proves procedure but does not prove the
function of the official keys.

## Certified voter registry

1. Obtain the certified test-voter file for ERT and the certified full actual
   registry for FTL and official voting. Obtain an independently supplied SHA-256
   and certified counts for every stable Post key for each file. Retain each
   transmittal separately from its uploaded file. A delta import alone does not
   certify the full actual registry.
2. For each imported event, confirm **Data → Advanced configurations → Voter file
   integrity** is **Required SHA-256**, as configured in the source before signing
   the package. If it is wrong, obtain a corrected higher approved revision. Older
   events without a policy retain **Optional SHA-256**. Optional imports without
   a verified declared hash cannot establish certified readiness evidence.
3. Import the certified test-voter file into ERT. Import the identical certified
   full actual registry into FTL and the official event. In each voters import
   drawer, enter that file's certified 64-character hexadecimal SHA-256. Required
   mode offers no bypass. The server also enforces the policy. A mismatch refuses
   the import; investigate the file and transmittal before submitting a new import.
4. Wait for successful completion. In the event's **Logs**, filter statement kind
   to `VotersImported` and open the entry. Retain the import and document IDs,
   filename, actual and declared hashes, import timestamp and counts by Post.
   Confirm the counts against the certified transmittal. A pending or failed
   import is not evidence of a completed certified registry.

A `FAILED` import task is terminal. Correct the cause and submit a new import
task; do not retry the old task. If voter commit or completed-import receipt
persistence failed, reconcile the current registry, Post counts and log evidence
against the transmittal before submitting the new import. A failed task does not
prove that no voter changes occurred.

For CLI import and readiness commands, see
[Certified imports and readiness checks](../../07-developers/02-cli/02-tutorials/07-cli-tutorials-readiness.md).
A locally computed hash can help diagnose a mismatch; it cannot replace the
independently certified expected value.

## Execute ERT and FTL

Confirm the approved **Seal at close** policy was imported into ERT and FTL
before opening voting, and publish the expected ballot designs for every Post.

Run the approved ERT script against the ERT event: authentication, ballot display,
casting, receipt verification, closing, sealing and a test tally. Retain the test
tally without publishing its results. Record expected outcomes and observations
using the [acceptance-stage ledger](../../07-developers/02-cli/02-tutorials/06-cli-tutorials-acceptance-stage.md).
Close every enabled channel, confirm every expected ballot box is **Sealed**,
export its evidence and then archive the ERT event.

Run FTL against its own event with the approved voters and credentials. Use the
actual SBEI quorum and opening/closing procedure. FTL does not run a tally.
Confirm every applicable election and channel is closed and every expected
publication-derived ballot box is **Sealed**; pending, failed or missing seals
need resolution. Export the evidence and archive the FTL event only after that
verification. Archiving is not a substitute for closing and sealing.

Issue credentials only in the FTL event's isolated realm. Confirm the issuance
does not alter official accounts or enrollment, and verify that a tested-event
ballot cannot be cast into the official event. Ordinary test ballots must never
be placed in the official event. Every ordinary official-event ballot blocks
the zero check, including records received during ballot review before casting.
Until an explicit practice-ballot designation and storage isolation are available,
run practice in isolated events. Official-event practice cannot currently be
certified by the readiness check.

## Verify readiness twice

Publish the official event and keep all its ordinary elections **Not started**.
Do not open official voting, create a tally session or run initialization that
creates such a session before this check. Future scheduled voting dates are
expected; actual voting must not have started or ended.

Run `step-cli step readiness-check` once after FTL and again when the lockdown
period starts. Each successful invocation records a separate signed `ReadinessVerified` entry
in the official event's existing **Logs**. Retain the JSON report and returned
log entry reference, then correlate that reference with the exported event log.
The report compares approved configuration content and approvers, certified
registry hashes and Post counts, official zero state, publication designs and
the tested event's closed and sealed state. The evidence includes current voter
counts (`current_voter_counts`), whether an import remains active
(`voter_import_in_progress`) and review-stage received ballots
(`received_ballot_count`). Current voter counts must match the certified import;
active imports and any official received ballot prevent a pass.

A failed check exits unsuccessfully and appends no `ReadinessVerified`. Resolve
its reported cause and repeat the check. Missing evidence cannot be treated as a
pass. A successful report describes its timestamped snapshot; it does not freeze
all later changes or certify enrollment, identity-provider configuration or
operational approvals. Repeat after any relevant correction and retain both the
correction record and the new report. Use the scheduled lockdown controls and
existing permissions for the approved restriction period; end lockdown before
the authorized tally.

## Evidence and sign-off

Keep distinct ERT and FTL evidence folders with these records:

- Approved scripts, expected outcomes, actual observations, acceptance ledger and
  witness identities.
- Package revisions, content and manifest hashes, verified approvers and event IDs.
- Certified voter transmittal, hashes, full Post counts and completed import IDs.
- Ceremony/custody transfer records, publication/design evidence and ballot-box
  seals; ERT tally evidence where applicable.
- CSV/PDF event-log exports, the after-FTL and at-lockdown readiness JSON and their
  signed log entry references.
- Signed minutes identifying the evidence hashes, approved exceptions, schedule,
  operator and witnesses.

Rehearse this procedure in an isolated environment using synthetic voters before
operational acceptance. Automated unit or fixture tests do not establish that a
live rehearsal or election-authority acceptance has taken place.
