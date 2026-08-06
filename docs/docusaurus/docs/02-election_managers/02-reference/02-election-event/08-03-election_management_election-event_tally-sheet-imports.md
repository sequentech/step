---
sidebar_position: 8.3
title: Tally Sheet Imports
---

# Tally Sheet Imports

Tally sheet imports let election administrators upload aggregated results from an external tabulator and review them as a batch before those results become active tally-sheet versions.

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

Each ballot box must include exactly one row for every required scalar field: `total_votes`, `total_valid_votes`, `implicit_invalid`, `explicit_invalid`, `total_blank_votes`, and `census`. It must also include one `candidate_votes` row for every candidate in the matched STEP contest. The set of field names is closed: any other `field` value is rejected as an `invalid_field` validation error, so a mistyped field name is reported rather than silently ignored.

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

Any other value is rejected as an `invalid_field` validation error.

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

- **party** names (`<Party name>`) are matched to STEP areas by exact area name. ES&S's physical precincts are not used as areas — a party maps to exactly one ballot style, which is what makes its whole-ballot counts a valid ballot count for every contest on that ballot. If no party name in the file matches any configured area, the import reports a single `ess_area_names_do_not_match_election_event` error up front rather than one failure per contest.
- contest `altId1` is matched to contest external ID
- candidate `altId1` is matched to candidate external ID
- only ES&S reporting group `1` is read: its values are summed across precincts into one result per party for the selected STEP channel, and all other reporting groups are ignored. Party-level data (`PrecinctParty`, `PrecinctPartySplit`, `CandidatePrecinctSplitVotes`) carries no reporting-group attribute of its own, so it cannot be scoped to one group. The import therefore refuses to run unless each precinct's party data adds up to exactly the requested group, rather than silently blending another channel's ballots (e.g. absentee) into it.
- each contest's `max_votes` is taken from the STEP contest config and cross-checked against the file's own `Contest@voteFor`. Every derived figure below is a selection-slot count divided by `max_votes`, so a wrong value corrupts all of them: a disagreement is reported as `ess_vote_for_does_not_match_contest_config`, and a contest the election event doesn't configure at all as `ess_contest_not_configured`. In both cases the contest is skipped rather than converted on a guessed bound.
- ES&S overvotes always become STEP implicit invalid votes. ES&S's `overVotes` is a selection-*slot* count, not a ballot count — an overvoted ballot always contributes its whole `max_votes` allotment to `overVotes` (confirmed by the EVS SOP and empirically) — so it is divided by `max_votes` to recover an overvoted-*ballot* count before being added to `implicit_invalid`; otherwise it could exceed `total_votes` and make `total_valid_votes` underflow.
- ES&S's own `blankVotes` figure is **not** used as STEP's `total_blank_votes`: per the ES&S EVS documentation, `blankVotes` is exactly `overVotes + underVotes`, not a genuine blank-ballot count, so it can't validate a "vote for N" contest correctly. `underVotes / max_votes` is used instead — `underVotes` is itself a selection-*slot* count (same issue as `overVotes` above), summing unused slots from both genuinely blank ballots and ballots with a valid partial selection, so dividing by `max_votes` only gives an *upper-bound approximation* of the blank-ballot count (exact for single-choice contests, and whenever every under-filled ballot happens to be entirely blank — ES&S's aggregate XML has no field distinguishing the two cases). This approximation is sanity-checked against the party's own whole-ballot blank count (summed from `PrecinctParty blanksCast` across precincts): every ballot blank on the whole ballot is necessarily blank in this contest too, so the approximation can never be smaller — a violation is reported as an `ess_blank_votes_below_precinct_minimum` validation error.
- `total_votes`/`total_valid_votes`/`census` are derived from each party's whole-ballot `PrecinctParty ballotsCast`, summed across precincts. Because a party maps to exactly one ballot style, that figure is an authoritative ballot count for every contest on that party's ballot — so vote-for-N contests, where candidate marks legitimately exceed the ballot count, convert correctly. `total_valid_votes` is that ballot count minus the invalid ballots recovered from `overVotes`.
- the import cross-checks the per-ballot accounting identity documented in the ES&S SOP: every one of a contest's `max_votes` selection slots, across every ballot cast for it, is either a candidate mark, an overvote slot, or an undervote slot, with no remainder (`sum(candidate_votes) + overVotes + underVotes == total_votes * max_votes`). A mismatch is reported as an `ess_vote_reconciliation_mismatch` validation error — it indicates a data-quality problem in the source file, not something STEP can correct on its own.
- the import also checks that ES&S's `overVotes` is an exact multiple of `max_votes` — required for the overvoted-ballot count used to compute `implicit_invalid` (see above) to be well-defined. A non-exact remainder is reported as an `ess_over_votes_not_divisible` validation error.
- a `(contest, party)` pair only produces a ballot box when it has genuine data (a candidate vote, an overvote, or an undervote). ES&S emits zero-valued entries for every party regardless of whether the contest appears on that party's ballot, so parties with no turnout in a contest are skipped rather than importing a flood of zero-valued ballot boxes.

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
