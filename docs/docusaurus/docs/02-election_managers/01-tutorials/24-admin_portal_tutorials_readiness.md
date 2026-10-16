---
id: admin_portal_tutorials_readiness
title: Election Readiness Rehearsal and Verification
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Use a separate tested election event to rehearse the approved procedure while
keeping the official event unopened. The readiness check compares these two
events and records technical evidence at one time. The election authority defines
the operating procedure, officer authorization, witnesses, key custody, account
isolation, retention and schedule; the report does not replace those approvals.

## Prepare the approved configuration

Assign an operator with `readiness-check` and the authorized officers and
witnesses required by the approved procedure. If the procedure uses lockdown,
plan its start and end with the existing scheduling controls. End lockdown before
an authorized tally that requires access to the restricted actions.

In the source configuration, choose **Required SHA-256** for voter-file integrity
and **Seal at close** before producing the approved tested and official packages.
Verify the selected source configuration or preset explicitly includes both
policies before signing. Policy fields present in the JSON participate in the
signed ballot-design digest, so changing them after admission can make publication
verification fail. Obtain corrected, signed and
approved packages at higher revisions when a policy correction is needed.

Prepare packages with distinct, increasing revisions and identical approved
content and verified approver identities. Manifest hashes may differ. Verify that
the configuration producer can issue these packages before beginning; ordinary
import freshness and duplicate protections still apply. Any later correction
must exceed the highest revision imported, including revisions used by other
rehearsal events. Do not edit signed content to label an event as a test.

Keep event identifiers and canonical signed area names with the evidence.
Registry counts use trimmed `area.name` values matched to
`manifest.content.ballot_designs[].area`.
Follow the [configuration import guide](../../07-developers/10-tutorials/06-import-election-event.md),
[publishing tutorial](./12-admin_portal_tutorials_publish-election.md),
[scheduling tutorial](./13-admin_portal_tutorials_scheduled-events.md) and
[seal runbook](./22-admin_portal_tutorials_ballot-box-seal-runbook.md).
Use the supported ceremony, export/import and trustee custody workflows required
by the approved procedure. A separate ceremony can rehearse the procedure but
does not prove operation with the official keys.

## Import the certified voter registry

1. Obtain the certified full registry and its independently supplied SHA-256 and
   counts for every canonical signed area name. Retain the transmittal separately
   from the file. Earlier rehearsals may use separately certified test data; the
   tested event used by the readiness comparison must match the official event's
   full certified registry. A delta file alone does not certify that registry.
2. Confirm **Data → Advanced configurations → Voter file integrity** is **Required
   SHA-256** in each imported event, as configured before signing. If it is wrong,
   obtain a corrected higher approved revision. Older events without a policy
   retain **Optional SHA-256**; an import without a verified declared hash cannot
   establish certified readiness evidence.
3. Import the identical certified full registry into the tested and official
   events. Enter the independently certified 64-character hexadecimal SHA-256.
   Required mode offers no bypass, and the server also enforces the policy. A
   mismatch refuses the import; investigate the file and transmittal before
   submitting a new import.
4. Wait for successful completion. In each event's **Logs**, filter statement kind
   to `VotersImported` and open the entry. Retain import/document IDs, filename,
   actual and declared hashes, import timestamp and counts by area. Confirm the
   counts against the transmittal. A pending or failed import is not evidence of
   a completed certified registry.

A `FAILED` task is terminal. Correct the cause and submit a new task; do not retry
the old task. If voter commit or receipt persistence failed, reconcile the
current registry, area counts and log evidence against the transmittal before
submitting a new import. A failed task does not prove that no voter changes occurred.

For commands, see [Certified imports and readiness checks](../../07-developers/02-cli/02-tutorials/07-cli-tutorials-readiness.md).
A locally computed hash can diagnose a mismatch but cannot replace the
independently certified expected value.

## Execute the rehearsal

Confirm the approved **Seal at close** policy was imported into the tested event
before opening voting, and publish the expected ballot designs for every area.
Run the approved script using the actual authorization and opening/closing
workflows. Include authentication, ballot display, casting, receipt verification
and any tally exercise required by the approved procedure. Record expected
outcomes and observations using the
[acceptance-stage ledger](../../07-developers/02-cli/02-tutorials/06-cli-tutorials-acceptance-stage.md).

Before readiness comparison, close every applicable election and voting channel,
and confirm every expected publication-derived ballot box is **Sealed**. Resolve
pending, failed or missing seals. Export and retain the evidence; archive the
tested event if the approved retention procedure calls for it. Archiving does not
replace closing and sealing.

Keep test credentials isolated from official accounts and enrollment. Verify a
tested-event ballot cannot be cast into the official event. Every ordinary
official-event ballot blocks zero verification, including records received during
ballot review before casting. Until explicit practice-ballot designation and
storage isolation are available, use isolated practice events; official-event
practice cannot currently be certified by this check.

## Verify readiness

Publish the official event and keep all its ordinary elections **Not started**.
Do not open official voting or create a tally session before checking readiness.
This also excludes initialization workflows that create tally sessions. Future
scheduled voting dates are expected; actual voting must not have started or ended.

In the Admin Portal, open the **official** election event and select **Check
readiness** beside its title. Select the completed **Tested event**, checking its
name and ID, then select **Run check**. Archived rehearsal events are included.
The action requires `readiness-check`; selecting events uses the existing
`election-event-read` permission and event permission labels. It remains available
during lockdown and does not require access to the Data or Logs tabs.

The dialog shows every check's outcome, the check time and, on success, the signed
log entry reference. Select **Download JSON report** to retain the complete report,
including evidence for failed checks. Changing the tested event clears the previous
result. If a request fails, check the official event's Logs before retrying: the
server may have recorded evidence before the response was lost.

Run this check at the points required by the approved procedure, such as after the
rehearsal or when a scheduled lockdown starts. The CLI alternative is
`step-cli step readiness-check`. Each successful invocation records a separate
signed `ReadinessVerified` entry in the official event's existing **Logs**. Retain
the JSON report and returned log entry reference, and correlate them with the
exported event log.

The report compares approved content and approvers, certified voter hashes and
area counts, official zero state, publication designs and tested close/seal state.
Evidence includes `current_voter_counts`, `voter_import_in_progress` and
`received_ballot_count`. Current counts must match the certified import; active
imports and any official received ballot prevent a pass.

A failed comparison shows the failed checks (and the CLI exits unsuccessfully)
and appends no `ReadinessVerified`. Resolve the reported cause and repeat. Missing
evidence cannot pass. A successful report is a snapshot, and does not freeze later changes or certify enrollment,
identity-provider configuration or operational approval. Repeat after relevant
corrections and retain the correction record and new report.

Labels and check descriptions use the existing tenant translation overrides.
For example, customize `logsScreen.readiness.fields.areas`,
`logsScreen.readiness.fields.counts_by_area`,
`logsScreen.readiness.fields.current_voter_counts` and
`logsScreen.readiness.checks.certified_voters` to use local terminology.
This changes the display; signed evidence and historical field names stay intact.

## Retain evidence and sign-off

Keep evidence for each rehearsal according to the approved procedure:

- Scripts, expected outcomes, observations, acceptance ledger and witness identities.
- Package revisions, content/manifest hashes, verified approvers and event IDs.
- Certified transmittals, voter-file hashes, full area counts and completed import IDs.
- Required ceremony/custody records, publication/design evidence, seals and tally evidence.
- CSV/PDF event-log exports, readiness JSON and corresponding signed entry references.
- Signed minutes identifying evidence hashes, approvals, exceptions, operator and witnesses.

Rehearse in an isolated environment using synthetic voters before operational
acceptance. Automated unit or fixture tests do not establish that a live
rehearsal or election-authority acceptance has taken place.
