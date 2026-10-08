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

Each ballot box must include exactly one row for every scalar field: `total_votes`, `total_valid_votes`, `implicit_invalid`, `explicit_invalid`, `total_blank_votes`, and `census`. It must also include one `candidate_votes` row for every candidate in the matched STEP contest.

Uploaded files are stored as normal STEP documents. The UI and CLI send a SHA-256 hash of the source file when they upload it, and the import actions verify that hash before parsing or persisting the import. The import record stores both the original source hash and the canonical CSV hash used for validation and review.

## Columns

`channel` is the STEP voting channel for the generated tally sheet versions, such as `PAPER`.

`area_name` must exactly match an existing area name in the election event.

`contest_external_id` must match the contest external ID configured in STEP. Because the canonical CSV format has no election column, **contest external IDs must be unique within an election event** for tally sheet import to resolve rows correctly. If an election event has multiple elections that reuse the same external ID for different contests, the import will fail with an ambiguous-match error — rename the duplicate external IDs before importing.

`field` must be one of:

- `candidate_votes`
- `total_blank_votes`
- `implicit_invalid`
- `explicit_invalid`
- `total_valid_votes`
- `total_votes`
- `census`

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
total_valid_votes = sum(candidate_votes) + total_blank_votes
total_votes = total_valid_votes + total_invalid
total_votes <= census
```

## ES&S Enhanced XML Mapping

For ES&S Enhanced XML uploads:

- precinct names are matched to STEP areas by exact area name
- contest `altId1` is matched to contest external ID
- candidate `altId1` is matched to candidate external ID
- only ES&S reporting group `1` is read: its values are summed per precinct into one result for the selected STEP channel, and all other reporting groups are ignored
- ES&S overvotes become STEP implicit invalid votes
- ES&S blank votes become STEP blank votes
- ES&S undervotes are treated with the overlap-safe rule `max(underVotes - blankVotes, 0)` before adding them to implicit invalid votes

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
