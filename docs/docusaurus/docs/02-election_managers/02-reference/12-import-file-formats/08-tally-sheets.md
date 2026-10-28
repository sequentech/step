---
id: import_format_tally_sheets
title: Tally Sheet File
sidebar_position: 8
description: "The format of the tally sheet file: the columns, the fields, the arithmetic rules, the validation errors and the review step."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The tally sheet file holds the results of paper, postal or in-person voting. Each row gives one
number for one area and one contest. You make the file yourself, for example from the count
sheets of the polling places. You import it in the election event, in the section **Tally sheet
imports** of the **Tally** tab. For the screen, see
[Tally Sheet Imports](../02-election-event/08-03-election_management_election-event_tally-sheet-imports.md).
For the steps of the tally, see [Run the Tally Ceremony](../../03-procedures/06-tally.md).

The import has two formats. This page describes **Canonical CSV**. The other format, **XML**, is
the results format of an external system. The platform converts that XML file to the canonical CSV
format and then runs the same checks.

You need these permissions: **Create Tally Sheet Import** (`tally-sheet-import-create`) to upload
a file, and **Review Tally Sheet Import** (`tally-sheet-import-review`) to approve or disapprove
it.

## Fields

The file has a header row. The order of the columns is not important. The columns are read by
name. The file must have all six columns.

| Column | Required | Allowed values or format | Default | Meaning | Example |
| --- | --- | --- | --- | --- | --- |
| `channel` | Required | `PAPER`, `POSTAL` or `IN_PERSON`. Use capital letters. | None | The voting channel of the results. It must be the same as the channel that you select in the import panel. | `PAPER` |
| `area_name` | Required | The **Name** of an area of the election event | None | The area of the results. The area must be linked to the contest. | `North` |
| `contest_external_id` | Required | The **External ID** of a contest | None | The contest of the results. | `contest-1` |
| `field` | Required | One of the values in the next table | None | The kind of number in the `value` column. | `census` |
| `candidate_external_id` | Only for `candidate_votes` | The **External ID** of a candidate of the contest | Empty | The candidate. Leave it empty for all other fields. | `candidate-1` |
| `value` | Required | A whole number, 0 or more | None | The number. | `1200` |

The import ignores spaces at the start and at the end of `channel`, `area_name`,
`contest_external_id`, `field`, `candidate_external_id` and `value`.

### Values of `field`

The list is closed. Any other value gives the error `invalid_field`.

| Value | Required | Meaning |
| --- | --- | --- |
| `census` | Yes | The number of voters who can vote in this area and contest. |
| `total_votes` | Yes | The number of ballots counted: valid, blank and invalid. |
| `total_valid_votes` | Yes | The number of valid ballots. It includes the blank votes. |
| `total_blank_votes` | Yes | The number of ballots that are blank in this contest. |
| `implicit_invalid` | Yes | The number of invalid votes caused by the counting rules, for example too many marks. |
| `explicit_invalid` | Yes | The number of invalid votes that the voter chose on purpose. |
| `blank_ballots` | No | The number of ballots that are blank in all contests of the area and channel. |
| `candidate_votes` | One row for each candidate | The votes of one candidate. |

The platform calculates `total_invalid` as `implicit_invalid` plus `explicit_invalid`. You do not
write it in the file.

### Rules for the numbers

The import checks these rules for each area and contest. It checks the area-contest pair in one
channel.

1. `total_votes` = `total_valid_votes` + `implicit_invalid` + `explicit_invalid`.
2. `total_votes` is not more than `census`.
3. The sum of the `candidate_votes` is between *A* and *A* x *M*. *A* is `total_valid_votes`
   minus `total_blank_votes`. *M* is the highest number of marks that one ballot can have.
   - *M* is the **Max Votes** of the contest. If the contest has no value, *M* is 1.
   - If the counting algorithm is `cumulative`, *M* is **Max Votes** multiplied by the number of
     checkboxes for each candidate.
   - For a normal contest where each voter chooses one candidate, the sum is exactly *A*.
4. `blank_ballots` (if you give it) has the same value on each row of the same area and channel.
   It is not more than the lowest `total_blank_votes` of the contests of that area. It is not less
   than the sum of the `total_blank_votes` of the contests, minus (the number of contests minus 1)
   multiplied by the highest `total_votes`. The platform also compares it with the approved
   sheets of the contests that are not in the file.
5. If these limits allow only one value, and you did not give `blank_ballots`, the platform fills
   in that value.

Other rules:

- Each candidate of the contest must have an **External ID**, and the External IDs must be unique
  in the contest.
- The file must have a `candidate_votes` row for each candidate of the contest, also if the
  votes are `0`.
- The External ID of the contest must be unique in the election event.
- You cannot import tally sheets for an acclaimed contest.

## Rules of the file

| Rule | Value |
| --- | --- |
| File type | `.csv` (or `.xml` for the other format). The separator of the CSV file is a comma. |
| Header | Required. |
| Encoding | UTF-8. |
| Order of rows | Not important. |
| Duplicate rows | Two rows with the same `field` for one area and contest give an error. The same is true for two `candidate_votes` rows for one candidate. |
| Size | 50 MB (52 428 800 bytes) at most. |
| Integrity check | The admin portal calculates the SHA-256 hash of the file. You do not type it. The server checks it. |
| Channel | One import has one channel. The channel in each row must be the same as the channel you select. |

## Example file

```csv
channel,area_name,contest_external_id,field,candidate_external_id,value
PAPER,North,contest-1,census,,1200
PAPER,North,contest-1,total_votes,,815
PAPER,North,contest-1,total_valid_votes,,790
PAPER,North,contest-1,total_blank_votes,,10
PAPER,North,contest-1,implicit_invalid,,9
PAPER,North,contest-1,explicit_invalid,,16
PAPER,North,contest-1,candidate_votes,candidate-1,480
PAPER,North,contest-1,candidate_votes,candidate-2,300
PAPER,South,contest-1,census,,900
PAPER,South,contest-1,total_votes,,600
PAPER,South,contest-1,total_valid_votes,,590
PAPER,South,contest-1,total_blank_votes,,5
PAPER,South,contest-1,implicit_invalid,,4
PAPER,South,contest-1,explicit_invalid,,6
PAPER,South,contest-1,candidate_votes,candidate-1,285
PAPER,South,contest-1,candidate_votes,candidate-2,300
```

In the area North: 790 + 9 + 16 = 815 (`total_votes`). 480 + 300 = 780, and 790 - 10 = 780
(candidate votes). 815 is less than 1200 (`census`). Check your own file with the same sums.

## What the import does

The import has two steps: a preview, and a save. Nothing changes in the results until a reviewer
approves the import.

1. In the election event, open the **Tally** tab. In **Tally sheet imports**, click **Import tally
   sheets**.
2. Select the **Format** (**Canonical CSV**) and the **Channel**.
3. Drop the file in the panel. If an earlier import has the same file, the portal shows the
   warning "This source file hash already appears in a previous tally sheet import." and the button
   **Open existing**. The warning does not stop you.
4. Click **Preview**. The platform reads the file, finds each area, contest and candidate, checks
   the rules and shows the ballot boxes. The panel shows the counts: **Imported**, **Changed**,
   **New**, **Unchanged**, **Conflicted** and **Errors**.
5. If the panel shows errors, correct the file and click **Preview** again. The button **Save
   import** stays disabled while there are errors.
6. Click **Save import**. The message "Tally sheet import created" shows. The import has the status
   **Pending review**. The platform makes a new, pending version of a tally sheet for each new
   or changed ballot box. It makes nothing for an unchanged ballot box.
7. A user with the review permission opens the import and clicks **Review**. The user compares the
   previous and the new values.
8. The user clicks **Approve** or **Disapprove**.

Each ballot box gets one of these labels in the preview: **New** (no approved tally sheet before),
**Changed** (the numbers are not the same as in the latest approved tally sheet) or **Unchanged**.

After **Approve**, the new tally sheets are approved. The platform marks as deleted all other versions of the same ballot box (same election, area, contest and channel), whatever their status. The tally then uses the approved sheets. After **Disapprove**,
the new tally sheets are disapproved and the results do not change.

If somebody approved another tally sheet for the same ballot box after you saved the import, the
import gets the status **Conflicted**. The message "Import has stale baseline conflicts" shows.
Nothing is approved. Import the file again.

### Statuses of an import

| Status | Meaning |
| --- | --- |
| **Pending review** | The import is saved. A reviewer must approve or disapprove it. |
| **Approved** | The new tally sheets are approved. |
| **Disapproved** | The new tally sheets are rejected. |
| **Failed validation** | The file had errors when the import was saved. The import has no tally sheets. |
| **Conflicted** | Another tally sheet changed after the import was saved. |

### Messages in the admin portal

| Message | Meaning and action |
| --- | --- |
| Select an import file before previewing it | Drop a file first. |
| Could not create upload URL, Could not upload import file | The upload failed. Try again. |
| Could not preview import, Preview response was empty | The preview failed. Read the message, correct the file and try again. |
| Tally sheet import created | The import is saved and waits for review. |
| Could not create import | The save failed. |
| Import approved, Import disapproved | The review is done. |
| Import has stale baseline conflicts | See the text above. |
| Could not review import | The review failed. |

## Validation errors

Each error has a code. The panel shows the message text for the code. The error shows the channel,
the area, the contest and the field when they are known.

| Code | Meaning | Action |
| --- | --- | --- |
| `invalid_csv_row` | A row has the wrong number of columns or cannot be read. | Correct the row. |
| `invalid_channel` | The `channel` is not `PAPER`, `POSTAL` or `IN_PERSON`. | Use one of these values in capital letters. |
| `selected_channel_mismatch` | The `channel` of a row is not the channel selected in the panel. | Select the same channel as in the file, or split the file. |
| `invalid_field` | The `field` is not in the list. | Correct the name. |
| `invalid_numeric_value` | The `value` is not a whole number of 0 or more. | Correct the value. |
| `missing_candidate_external_id` | A `candidate_votes` row has no `candidate_external_id`. | Add the External ID. |
| `duplicate_candidate_votes` | Two rows give the votes of one candidate. | Remove one row. |
| `duplicate_scalar_field` | Two rows give the same field for one area and contest. | Remove one row. |
| `missing_scalar_field` | A required field has no row. | Add the row. |
| `unresolved_ballot_box` | The platform did not find the area, the contest or a candidate, or the area is not linked to the contest, or a candidate has no External ID, or a candidate has no `candidate_votes` row. | Read the message. Correct the file or the election event. |
| `tally_sheet_not_allowed_for_acclaimed_contest` | The contest is acclaimed. | Remove the contest from the file. |
| `unknown_counting_algorithm` | The counting algorithm of the contest is not known. | Correct the contest. |
| `invalid_total_invalid` | `total_invalid` is not `implicit_invalid` + `explicit_invalid`. | Correct the numbers. |
| `invalid_total_valid_votes` | The candidate votes are outside the allowed limits (rule 3). | Correct the numbers. |
| `invalid_total_votes` | `total_votes` is not `total_valid_votes` + the invalid votes (rule 1). | Correct the numbers. |
| `total_votes_exceeds_census` | `total_votes` is more than `census` (rule 2). | Correct the numbers. |
| `inconsistent_blank_ballots` | The rows of one area and channel give different values for `blank_ballots`. | Use one value. |
| `blank_ballots_out_of_bounds` | `blank_ballots` is outside the limits (rule 4). | Correct the value. |

The messages of `unresolved_ballot_box` are:

- `Area '<name>' not found`
- `Contest external id '<id>' not found`
- `Area '<name>' is not assigned to contest external id '<id>'`
- `Contest external id '<id>' has candidate(s) without an external_id required for import: <names>`
- `Contest external id '<id>' has duplicate candidate external_id value(s): <ids>`
- `Import for contest external id '<id>' is missing candidate_votes rows for candidate external_id value(s): <ids>`
- `Candidate external id '<id>' not found in contest external id '<id>'`

The text of `tallysheet.inputError` in the admin portal gives the message for these codes:
`invalid_total_valid_votes`, `total_votes_exceeds_census`, `invalid_total_invalid`,
`invalid_total_votes`, `unknown_counting_algorithm`, `inconsistent_blank_ballots` and
`blank_ballots_out_of_bounds`. For the other codes, the portal shows the English message of the
server.

A file larger than 50 MB is refused with the error "Document ... is too large". A hash that does
not match is refused with the error "Uploaded source SHA-256 mismatch".

The portal shows the error message of the failed request. The server answers with status 413 for a file that is too large and with status 400 for a hash mismatch.

## Related pages

- [Tally Sheet Imports](../02-election-event/08-03-election_management_election-event_tally-sheet-imports.md): the **Tally sheet imports** section.
- [Run the Tally Ceremony](../../03-procedures/06-tally.md): the tally uses the approved tally sheets.
- [Area File](04-areas.md): the areas that the tally sheet file names.
- [Import File Formats](index.md): all import files.
