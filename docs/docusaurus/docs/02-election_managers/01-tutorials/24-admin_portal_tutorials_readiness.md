---
id: admin_portal_tutorials_readiness
title: Election Readiness Rehearsal and Verification
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Use three election events in the same tenant: **ERT** for the readiness rehearsal,
**FTL** for final testing, and **official** for enrollment and actual voting. Label
the tested events in their display names without changing signed content. The
official event must be imported and published, with its certified registry loaded,
before enrollment starts; its voting stays unopened until the first voting day.
The readiness check compares FTL with the official event and records technical
evidence at one time. The election authority defines
the operating procedure, officer authorization, witnesses, key custody, account
isolation, retention and schedule; the report does not replace those approvals.

## Prepare the approved configuration

Assign an operator with `readiness-check` and the authorized officers and
witnesses required by the approved procedure. Record the authority's decisions on
the separate-event re-zero, custody, selected voter accounts, schedule exceptions
and test tallies before execution. Keep separate ERT and FTL evidence folders.

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
by the approved procedure. To prove that the official keys decrypt, perform one
ceremony on the airgap computer and carry its keys into all three events using
event exports that include the bulletin board, subject to the approved custody
decision. Retain the ceremony and import-verification evidence. A separate
ceremony can rehearse the procedure but does not prove operation with the
official keys.

## Set and verify the official schedule

In the **official** event's **Scheduled Events**, enter the start and end dates
for ERT, FTL, lockdown and voting for every area, or use **Import schedule**.
Check each area's timezone and export the schedule. FTL must be no later than
seven days before voting; resolve every existing seven-day warning or retain the
authority's approval of the exception in the FTL minutes.

Retain `LifecycleWindowChanged` entries as the scheduled windows open and close.
Set **Start Lockdown Period** for the end of FTL and **End Lockdown Period** before
the authorized official tally; the proposed period ends when voting closes in
every area. **Lockdown Status** is read-only and changes through these scheduled
events. The operator must run the readiness check again when lockdown starts.

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

## Execute ERT

1. Import and publish the ERT package with the same approved content and approvers
   as the official event, using a distinct increasing revision. Retain its import
   and publication log entries and package hashes.
2. Import the certified **test** voter file into ERT using the integrity procedure
   above. Its hash and area counts must match its own certified transmittal; this
   file can differ from the official registry.
3. Confirm the imported **Seal at close** policy and the expected published ballot
   designs for every area. Put the approved keys in place, open ERT through the
   existing authorization workflow, and open an OPS-PQ acceptance ledger before
   voters authenticate or vote. Run the approved script, including authentication,
   ballot display, casting and receipt verification. Record expected results and
   observed results in the ledger.
4. Close every applicable election and voting channel. Confirm every expected
   publication-derived ballot box is **Sealed**; resolve missing, failed or pending
   seals. Run the ERT tally with the trustees, compare results by contest and area
   with the script, and generate reports. Keep the results as evidence without
   publishing them.
5. Export ERT's CSV/PDF logs and reports, retain its seals and acceptance ledger,
   then archive ERT. The auditor signs ERT minutes listing each evidence file and
   its SHA-256. The event must remain closed, sealed and archived with its logs.

## Execute FTL

1. Recheck the seven-day lead and any approved exception. Import and publish the
   FTL revision of the same approved content, retaining its package and schedule
   evidence. Keep the official event unopened.
2. Import the **same certified full registry** into FTL and the official event
   using the same certified hash. Match both area-count maps to the transmittal.
   Give selected voters accounts and credentials in FTL only; their official
   accounts remain untouched.
3. Each area's authorized officers initialize and open FTL using the existing
   quorum workflow. Open a separate OPS-PQ ledger before the selected voters
   authenticate or vote, execute the script, and record which areas started and
   completed. Close every election and channel and verify every expected box is
   sealed under **Seal at close**. Do not run an FTL tally.
4. With the auditor watching, compare FTL against the official event as described
   below. Retain the passing JSON and its signed `ReadinessVerified` reference.
   Archive FTL after the check; keep its logs and seals.
5. Wait for the official event's scheduled **Start Lockdown Period**. Verify its
   lockdown banner above the event's tabs and retain `LockdownChanged`. The banner
   shows the recorded start and the scheduled end in the event's primary timezone
   when those records are available to the administrator. If no end is scheduled,
   it says so. Run the readiness check a second time using the CLI, because the Data
   tab is unavailable during lockdown. Retain a separate JSON report and the second
   `ReadinessVerified` reference. Both the lockdown state and the second check must pass.
6. Export the official and FTL CSV/PDF logs. Match both readiness records to the
   saved JSON outputs. The auditor signs the FTL minutes listing every evidence
   file and its SHA-256, including the schedule and any approved exception.

Use the existing
[acceptance-stage ledger](../../07-developers/02-cli/02-tutorials/06-cli-tutorials-acceptance-stage.md)
for each event. For example, after ERT is open and before its voters start:

```bash
mkdir -p ert-evidence ftl-evidence
step-cli acceptance init --template voting --output ert-evidence/voting-stage.yaml
```

Edit the stage to include the approved script's checks, then open its ledger:

```bash
step-cli acceptance open ert-evidence/voting-stage.yaml \
  --ledger ert-evidence/voting-ledger.jsonl \
  --election-event-id "$ERT_EVENT_ID"
```

After voting, run the automatic checks with the participating test usernames and
the receipt Ballot IDs, then record all witnessed checks before getting the verdict:

```bash
step-cli acceptance run ert-evidence/voting-ledger.jsonl \
  --voters-file ert-voters.txt --ballot-ids-file ert-ballot-ids.txt
step-cli acceptance record ert-evidence/voting-ledger.jsonl \
  --check voting.ballot-displayed --outcome pass --witness "$WITNESS" \
  --note "Approved ballot display checks completed"
step-cli acceptance verdict ert-evidence/voting-ledger.jsonl \
  --report ert-evidence/voting-report.md
```

Record a pass only when the witness observed it; record failures with their
evidence. Repeat with a new stage and ledger in `ftl-evidence`, `$FTL_EVENT_ID`, and
the FTL participants and receipt IDs. Keep each ledger head with its signed minutes.
Use the separate certified-import and readiness command examples in the
[CLI guide](../../07-developers/02-cli/02-tutorials/07-cli-tutorials-readiness.md).
These commands use the existing OPS-PQ implementation; they do not cast ballots
for the voters.

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

In the Admin Portal, open the **official** election event's **Data** tab and select
**Check readiness** alongside the other action buttons. Select the completed
**Tested event**, checking its name and ID, then select **Run check**. Archived
rehearsal events are included.
The action requires `readiness-check`; selecting events uses the existing
`election-event-read` permission and event permission labels. Access follows the
Data tab's existing permission and lockdown rules; use the CLI when that tab is
hidden. Running the check does not require access to the Logs tab.

The dialog shows every check's outcome, the check time and, on success, the signed
log entry reference. Select **Download JSON report** to retain the complete report,
including evidence for failed checks. Changing the tested event clears the previous
result. If a request fails, check the official event's Logs before retrying: the
server may have recorded evidence before the response was lost.

Run this check after FTL closes and again when scheduled lockdown starts, as above.
The CLI alternative is
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
