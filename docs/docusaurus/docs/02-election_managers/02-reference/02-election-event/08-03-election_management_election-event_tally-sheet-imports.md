---
sidebar_position: 8.3
title: Tally Sheet Imports
---

# Tally Sheet Imports

Tally sheet imports let election administrators upload precinct-level results from an external tabulator and review them as a batch before those results become active tally-sheet versions.

An import is election-event scoped. One uploaded file can produce many pending tally sheet versions across areas, contests, elections, and a selected voting channel. Saving an import does not change active results. Active tally-sheet results change only after the generated versions are approved.

In the Admin Portal, open an election event and go to **Tally**. The **Tally sheet imports** section appears above the tally ceremony list for users with `tally-sheet-import-view`.

## Permissions

The feature uses these permissions:

- `tally-sheet-import-view` to see import history, import detail, generated import items, source-file metadata, and imported-version source links
- `tally-sheet-import-create` to upload, preview, and save imports for review
- `tally-sheet-import-review` to approve or disapprove a saved import
- `tally-recount-execute` to manually trigger a recount of a completed tally session

## Supported File Formats

STEP uses a vendor-neutral canonical CSV format for tally sheet imports. ES&S Enhanced XML uploads are converted server-side into this CSV format before validation, preview, saving, review, and audit.

The canonical CSV is a long format with one row per field value:

```csv
channel,area_name,contest_external_id,field,candidate_external_id,value
PAPER,WARD 15 SUB 01,1,total_votes,,111
PAPER,WARD 15 SUB 01,1,total_valid_votes,,111
PAPER,WARD 15 SUB 01,1,implicit_invalid,,0
PAPER,WARD 15 SUB 01,1,explicit_invalid,,0
PAPER,WARD 15 SUB 01,1,total_blank_votes,,0
PAPER,WARD 15 SUB 01,1,census,,111
PAPER,WARD 15 SUB 01,1,candidate_votes,1,2
PAPER,WARD 15 SUB 01,1,candidate_votes,2,1
```

Rows are grouped into ballot boxes by:

```text
channel + area_name + contest_external_id
```

The selected import channel must match every `channel` value in the CSV.

Each ballot box must include exactly one row for every required scalar field: `total_votes`, `total_valid_votes`, `implicit_invalid`, `explicit_invalid`, `total_blank_votes`, and `census`. It must also include one `candidate_votes` row for every candidate in the matched STEP contest. Any other `field` value (e.g. `over_votes`/`under_votes`, written by the ES&S import path — see below) is accepted without validation and carried through as extra data on the imported ballot box, so new source-specific fields don't need a canonical CSV format change to be picked up.

Uploaded files are stored as normal STEP documents. The UI and CLI send a SHA-256 hash of the source file when they upload it, and the import actions verify that hash before parsing or persisting the import. The import record stores both the original source hash and the canonical CSV hash used for validation and review.

## Columns

`channel` is the STEP voting channel for the generated tally sheet versions, such as `PAPER`.

`area_name` must exactly match an existing area name in the election event.

`contest_external_id` must match the contest external ID configured in STEP. Because the canonical CSV format has no election column, **contest external IDs must be unique within an election event** for tally sheet import to resolve rows correctly. If an election event has multiple elections that reuse the same external ID for different contests, the import will fail with an ambiguous-match error — rename the duplicate external IDs before importing.

`field` is usually one of the required scalar fields or `candidate_votes`:

- `candidate_votes`
- `total_blank_votes`
- `implicit_invalid`
- `explicit_invalid`
- `total_valid_votes`
- `total_votes`
- `census`

Any other value (e.g. `over_votes`, `under_votes`) is accepted as unvalidated extra data — see the note above.

`candidate_external_id` is required only when `field` is `candidate_votes`. It must match a candidate external ID in the matched contest.

`value` must be a non-negative integer.

## Vote Buckets

Candidate vote rows are summed into candidate results.

`total_blank_votes` is treated as part of valid votes.

`explicit_invalid` is for an intentional invalid-vote option in STEP. ES&S Enhanced XML does not have an equivalent concept, so ES&S imports set it to `0`.

`implicit_invalid` is for invalid votes caused by tally rules, such as overvotes and non-blank undervotes.

The importer validates these invariants per ballot box:

```text
total_invalid = implicit_invalid + explicit_invalid
non_blank_valid_votes = total_valid_votes - total_blank_votes
non_blank_valid_votes <= sum(candidate_votes) <= non_blank_valid_votes * max_marks_per_ballot
total_votes = total_valid_votes + total_invalid
total_votes <= census
```

A voter may be allowed to mark more than one candidate per ballot, so `sum(candidate_votes)` isn't required to equal the ballot count — it only has to fall within the range a valid ballot for this contest could produce. `max_marks_per_ballot` is `1` for ordinary single-choice contests, the contest's `max_votes` for plurality-at-large "vote for N" contests, and `max_votes` multiplied by the cumulative-voting checkbox limit for cumulative contests.

## ES&S Enhanced XML Mapping

For ES&S Enhanced XML uploads:

- precinct names are matched to STEP areas by exact area name
- contest `altId1` is matched to contest external ID
- candidate `altId1` is matched to candidate external ID
- only ES&S reporting group `1` is read: its values are summed per precinct into one result for the selected STEP channel, and all other reporting groups are ignored
- ES&S overvotes always become STEP implicit invalid votes. ES&S's `overVotes` is a selection-*slot* count, not a ballot count — an overvoted ballot always contributes its whole `max_votes` allotment to `overVotes` (confirmed by the EVS SOP and empirically). For contests where ballots-cast is reported per contest and precinct (the `ContestReportingGroupVotes` XML variant), `overVotes` is divided by `max_votes` to recover an overvoted-*ballot* count before it's added to `implicit_invalid`; otherwise it could exceed `total_votes` and make `total_valid_votes` underflow. The other ES&S variant derives `total_votes` from candidate marks/blank votes instead (see below), so it isn't exposed to that underflow and uses the raw `overVotes` count directly.
- ES&S undervotes only become implicit invalid votes (via the overlap-safe rule `max(underVotes - blankVotes, 0)`) when the contest requires a minimum number of selections (`min_votes > 0`) — undervoting an otherwise-optional contest is never invalid
- ES&S's own `blankVotes` figure is **not** used as STEP's `total_blank_votes` for contests where ballots-cast is reported per contest and precinct (the `ContestReportingGroupVotes` XML variant): per the ES&S EVS documentation, `blankVotes` is exactly `overVotes + underVotes`, not a genuine blank-ballot count, so it can't validate a "vote for N" contest correctly. `underVotes` alone is used instead, which reconciles exactly for single-choice contests. The other ES&S variant (candidate-reporting-group) doesn't have this problem — its blank figure comes from the precinct's own `blanksCast`, a genuine (if precinct-wide) blank-ballot count — so it keeps using that.
- `total_votes`/`total_valid_votes` are derived from ES&S's own ballots-cast figure when it's reported per contest *and* precinct (the `ContestReportingGroupVotes` XML variant), so vote-for-N contests (where candidate marks legitimately exceed the ballot count) convert correctly. The other ES&S variant only reports ballots cast per precinct, shared across every contest on the ballot — not a valid ballot count for a contest that doesn't appear on every ballot style in that precinct (e.g. a ward- or school-board-specific race). For that variant, `total_valid_votes` is derived from candidate marks plus blank votes instead, the only figures actually scoped to that contest and precinct; `census` always uses the precinct-wide ballots cast regardless of variant.
- for the `ContestReportingGroupVotes` variant, the import additionally writes `over_votes`/`under_votes` rows carrying ES&S's raw counts, and cross-checks the per-ballot accounting identity documented in the ES&S SOP: every one of a contest's `max_votes` selection slots, across every ballot cast for it, is either a candidate mark, an overvote slot, or an undervote slot, with no remainder (`sum(candidate_votes) + over_votes + under_votes == total_votes * max_votes`). A mismatch is reported as an `ess_vote_reconciliation_mismatch` validation error — it indicates a data-quality problem in the source file, not something STEP can correct on its own.
- for the `ContestReportingGroupVotes` variant, the import also checks that `over_votes` is an exact multiple of `max_votes` — required for the overvoted-ballot count used to compute `implicit_invalid` (see above) to be well-defined. A non-exact remainder is reported as an `ess_over_votes_not_divisible` validation error.

## Import Lifecycle

Preview validates the file and compares each incoming ballot box against the latest approved version for the same election, area, contest, and channel. The preview shows new, changed, unchanged, and validation-error counts.

Save creates an immutable import record and generated `PENDING` tally sheet versions for changed or new ballot boxes. Unchanged ballot boxes remain visible in the import detail but do not need duplicate generated versions.

Approve all approves the generated versions if the baseline approved tally sheets have not changed and no newer non-deleted version exists for any affected ballot box. If a conflict is found, approval is stopped and the conflicted rows are marked.

Disapprove all marks the generated versions and import items as disapproved. Existing approved tally sheet versions remain active.

Import records can have these statuses:

- `PENDING_REVIEW`
- `APPROVED`
- `DISAPPROVED`
- `FAILED_VALIDATION`
- `CONFLICTED`

Import items can have these statuses:

- `PENDING_REVIEW`
- `APPROVED`
- `DISAPPROVED`
- `CONFLICTED`

The first implementation reviews a whole import at once. There is no selected-row or partial approval flow.

If the election event automatic recount policy is enabled, approving an import starts recount tasks only for elections where approved import items generated new tally sheet versions. Previewing, saving, disapproving, or approving an unchanged-only import does not start a recount.

The source file remains linked from the import history for later download and audit.

## Import Diff And Traceability

Each import item represents one ballot box, identified by election, area, contest, and channel. The detail view stores and shows two canonical per-ballot-box CSV snippets:

- `previous_csv`, rendered from the latest approved tally sheet at preview time
- `incoming_csv`, rendered from the imported source data

Rows are classified as:

- `NEW` when there is no approved baseline for the ballot box
- `CHANGED` when the imported content differs from the approved baseline
- `UNCHANGED` when the imported content matches the approved baseline

Only `NEW` and `CHANGED` rows generate pending tally sheet versions. Imported tally sheet versions keep an `import_id` reference so the version history can link back to the source import, show import status, and download the original XML or CSV source file.

## CLI

The same flow is available in `step-cli` under `cli step tally-sheet`:

- `import-preview`
- `import-create`
- `import-review`
- `import-list`
- `import-show`
- `import-download-source`
- `convert-ess-xml`
- `recount`

Use `convert-ess-xml` to convert an ES&S Enhanced XML file into canonical CSV for offline inspection before upload.
