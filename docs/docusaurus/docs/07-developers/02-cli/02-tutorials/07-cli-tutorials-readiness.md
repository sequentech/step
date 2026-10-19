---
id: cli_tutorials_readiness
title: Certified Imports and Readiness Checks
position: 7
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Configure the intended tenant using `step-cli step config` as described in the
[getting-started guide](./01-cli-tutorials-getting-started.md). The account needs
voter import permission for imports and `readiness-check` for readiness checks.
Use the [manager runbook](../../../02-election_managers/01-tutorials/24-admin_portal_tutorials_readiness.md)
for the rehearsal, witnesses, custody, schedules and evidence required around
these commands.

## Import a certified voter file

For signed configuration packages, configure **Required SHA-256** and **Seal at
close** in the source before producing and approving the tested and official
packages. Verify the selected source configuration or preset explicitly includes
both policies before signing. JSON policy fields participate in the signed ballot-design digest; changing an admitted
policy can invalidate publication verification. Obtain corrected higher signed
and approved revisions when the source policy needs correction.

The expected hash must come from the independent certified transmittal. Import
the identical certified full registry into the tested and official events.
Earlier rehearsals may use separately certified test data; the event selected
for readiness comparison must match the official registry:

```bash
step-cli step import-voters \
  --election-event-id "$OFFICIAL_EVENT_ID" \
  --file-path certified-voters.csv \
  --sha256 "$CERTIFIED_SHA256"

step-cli step import-voters \
  --election-event-id "$TESTED_EVENT_ID" \
  --file-path certified-voters.csv \
  --sha256 "$CERTIFIED_SHA256"
```

`--sha256` supplies the declared 64-character hexadecimal SHA-256. The server
compares it to the exact uploaded bytes before writing voters. An event with
`presentation.file_integrity_policy = "REQUIRED_SHA256"` refuses missing hashes.
Older events default to `"OPTIONAL"`; an import without a verified declared hash
cannot establish certified readiness evidence. The command waits for task
completion and exits nonzero on failure or timeout. Retain the successful
`VotersImported` log entry and independently verify its area counts against the
transmittal. Count-map keys are trimmed `area.name` values matched to
`manifest.content.ballot_designs[].area`. Do not substitute the last delta file
for a full certified registry.

A `FAILED` task is terminal. Fix the reported cause and submit a new import
command to create a new task; do not retry the old task. A failure during voter
commit or completed-import receipt persistence requires reconciling the current
registry, area counts and log evidence against the certified transmittal before
submitting the new import. The failed task alone does not prove that no voter
changes occurred.

## Compare the tested and official events

After closing and sealing the tested event and publishing the unopened official
event, run:

```bash
step-cli step readiness-check \
  --official "$OFFICIAL_EVENT_ID" \
  --tested "$TESTED_EVENT_ID" \
  > readiness-after-rehearsal.json
```

Both events must be distinct and belong to the configured tenant. The server
derives the actor from the authenticated account. The command writes the JSON
report to stdout, diagnostics to stderr and exits nonzero when a check fails or
the signed record cannot be confirmed. Inspect the exit status before accepting
the report as a pass.

The report contains `passed`, `checked_at`, named `checks` with their verdicts
and explanations, `evidence` for both events and `log_entry_id` on success. It
compares approved package content and approvers, certified voter hashes and area
counts, official zero state, official publication designs and tested close/seal
completeness. Each event's evidence includes `current_voter_counts`,
`voter_import_in_progress` and `received_ballot_count`. Current voter counts must
match the certified import, and no import may remain active. Any ordinary
official-event ballot, including a record received at ballot review before
casting, blocks the zero check. Until an explicit practice-ballot designation
and storage isolation are implemented, use isolated practice events;
official-event practice cannot currently be certified. The report is a snapshot
at `checked_at`.

Both events' active publications must together contain every ballot design in
their signed manifest. Separate publications per election are supported; their
references are listed in `publication_ids`. An incomplete set cannot establish
readiness, even when every published design is approved. The tested event's
required seals come from these publications; generated but unpublished drafts
and retired styles do not add required boxes.

A successful invocation appends `ReadinessVerified` to the official event log;
a failed check does not. In the existing **Logs** tab, filter statement kind to
`ReadinessVerified`, open the entry and correlate its report with the returned
reference. Export CSV/PDF logs through the existing export action. Keep the
complete JSON alongside the exports.

For the ERT/FTL procedure, compare the closed, sealed FTL event with the official
event, then archive FTL. When the official event's scheduled lockdown starts,
confirm its read-only lockdown status and retain `LockdownChanged`, then run:

```bash
step-cli step readiness-check \
  --official "$OFFICIAL_EVENT_ID" \
  --tested "$TESTED_EVENT_ID" \
  > readiness-at-lockdown.json
```

Here `$TESTED_EVENT_ID` must still identify FTL. Inspect the command exit status
and retain the second `ReadinessVerified` reference alongside the first report
and both events' exported logs in the FTL evidence folder. The readiness verdict
does not require lockdown; verify the reported official `locked_down` value is
`true` for this second execution. ERT has its own evidence folder, including its
tally; FTL has no tally.

An intentional new invocation is a new snapshot. Neither invocation opens voting, clears ballots, changes seals or
replaces the operational approval procedure.
