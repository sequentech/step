---
id: import_format_applications
title: Applications File
sidebar_position: 5
description: "The format of the applications file: the twelve columns, the JSON in applicant_data, the allowed statuses and what the import does."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The applications file holds the applications of the voters that enrolled through the enrollment
form of the election event. You make the file with **Export** on the **Approvals** tab. You import
it with **Import** on the same tab. Do not make the file by hand.

Use the import to restore applications that you exported earlier, or to copy them to a new
election event. For the screen, see
[Approvals](../02-election-event/14-election_management_election-event_approvals.md#import-file).

:::caution CAUTION: tenant and election event in the applications file
The import reads the tenant ID and the election event ID from each row. It does not use the
election event that you have open. Import the file only into the election event where you
exported it. If you import it again, you get a second copy of each application.
:::

## Fields

The file has a header row and twelve columns. The order of the columns is fixed. The import reads
the columns by position, not by name.

| Column | Position | Required | Allowed values or format | Default | Meaning | Example |
| --- | --- | --- | --- | --- | --- | --- |
| `id` | 1 | Optional | A UUID | None | The ID of the application. The import does not use it. It gives each application a new ID. | `4f1c0b0e-0000-4000-8000-000000000001` |
| `created_at` | 2 | Optional | Date and time with a time zone offset | The first date of the system (1970) | When the application was made. | `2026-10-01 09:30:00 +00:00` |
| `updated_at` | 3 | Optional | Same as `created_at` | The first date of the system (1970) | When the application changed. | `2026-10-02 10:00:00 +00:00` |
| `tenant_id` | 4 | Required | A version 4 UUID | None | The tenant of the application. A row with a value that is not valid is skipped. | `90505c8a-23a9-4cdf-a26b-4e19f6a097d5` |
| `election_event_id` | 5 | Required | A version 4 UUID that exists | None | The election event of the application. | `c2a1d6f0-0000-4000-8000-000000000002` |
| `area_id` | 6 | Required, see the notes | A version 4 UUID | None | The area of the application. | `7d3e5a10-0000-4000-8000-000000000003` |
| `applicant_id` | 7 | Required | Text | None | The ID that the enrollment form gave to the applicant. | `applicant-001` |
| `applicant_data` | 8 | Required | A JSON object | `null` | The data that the applicant typed in the form. | See below. |
| `labels` | 9 | Optional | Text (JSON) | Empty | Labels that the form sent with the application. | `{}` |
| `annotations` | 10 | Optional | Text (JSON) | Empty | The data that the platform adds during the verification. | See below. |
| `verification_type` | 11 | Required | `AUTOMATIC` or `MANUAL` | None | Who decided: the platform (`AUTOMATIC`) or an administrator (`MANUAL`). | `AUTOMATIC` |
| `status` | 12 | Required | `PENDING`, `ACCEPTED` or `REJECTED` | None | The state of the application. | `ACCEPTED` |

Notes:

- The date format is the one that the export writes: `YYYY-MM-DD HH:MM:SS` followed by a space and
  the offset, for example `2026-10-01 09:30:00 +00:00`. If the date cannot be read, the import
  uses the first date of the system and does not show an error.
- The export writes an empty `area_id` for an application without an area.
  A row with an empty `area_id` stops the import with an error that starts with `invalid UUID ''`.
- The import does not check the values of `verification_type` and `status` against the list. The
  admin portal shows only the values in the table. Use only these values.
- The export does not write the permission label of an application. After an import the permission
  label is empty.

### The column `applicant_data`

`applicant_data` is a JSON object. Each key is the name of a field of the enrollment form. Each
value is the text that the applicant typed. The keys depend on the user profile of the election
event. The form can send first name, last name, email address, date of birth, and any attribute of
the user profile.

```json
{"firstName":"Ana","lastName":"Lopez","email":"voter001@example.com","dateOfBirth":"1990-01-31","username":"voter001"}
```

- The platform adds the key `username` when it saves the application. Its value is the user name
  of the voter that matched, or an empty text if no voter matched.
- The **Approvals** list shows one column for each attribute of the user profile. It reads the
  value from the key of the same name in `applicant_data`.
- If the JSON is not valid, the import saves `null` and does not show an error.

### The column `annotations`

The verification keeps its working data here. Keys that you can find:

| Key | Meaning |
| --- | --- |
| `session_id`, `credentials` | Data of the session of the form. |
| `search-attributes` | The attributes that the platform uses to search for a voter. Comma separated. |
| `unset-attributes` | The attributes of the voter that must be empty before the platform accepts the application. |
| `update-attributes` | The attributes of the voter to update, separated by commas. When an administrator confirms the application, the platform copies these keys from `applicant_data` to the voter. `email`, `firstName` and `lastName` set the voter's own fields. |
| `fields_match` | For each searched field: `true` if the application and the voter have the same value. |
| `mismatches` | The number of fields that differ. |
| `manual_verify_reason` | The reason why an administrator must check the application. |
| `rejection_reason` | `insufficient-information`, `no-matching-voter`, `voter-already-approved` or `other`. |
| `rejection_message` | The text of the rejection. Required for the reason `other`. |
| `verified_by` | The administrator who decided. |

## Rules

| Rule | Value |
| --- | --- |
| File type | `.csv`. The separator is a comma. |
| Header | The export writes a header row. The import does not skip it. It skips it because the value `tenant_id` is not a valid UUID. A file without a header row works. |
| Encoding | UTF-8. |
| Quotes | The export puts the JSON columns in double quotes and doubles each quote inside. Do not change this. |
| Order of columns | Fixed. Do not move or remove a column. |
| Size | The size is limited by the deployment setting for file uploads. |
| Election filter | If you export from the **Approvals** tab of one election, the file has only the applications of that election. |

## Example file

```csv
id,created_at,updated_at,tenant_id,election_event_id,area_id,applicant_id,applicant_data,labels,annotations,verification_type,status
4f1c0b0e-0000-4000-8000-000000000001,2026-10-01 09:30:00 +00:00,2026-10-01 09:30:05 +00:00,90505c8a-23a9-4cdf-a26b-4e19f6a097d5,c2a1d6f0-0000-4000-8000-000000000002,7d3e5a10-0000-4000-8000-000000000003,applicant-001,"{""firstName"":""Ana"",""lastName"":""Lopez"",""email"":""voter001@example.com"",""username"":""voter001""}","{}","{""search-attributes"":""firstName,lastName"",""mismatches"":0}",AUTOMATIC,ACCEPTED
```

## What the import does

1. The admin portal uploads the file and asks for the **Integrity Check (SHA-256)**. The task
   checks the hash if you typed one.
2. The task starts. Look at the **Tasks** tab to see the result.
3. For each row, the task reads the 12 columns. It skips a row if `tenant_id` is not a valid UUID.
4. The task adds all the other rows in one database transaction. If one row is wrong, the task
   adds nothing and the status is **Failed**.
5. Each application gets a new ID. The dates, the data, the status and the type stay as in the file.
6. The import does not send messages to the applicants. It does not create or change voters.

Known effect: the import stores `labels` and `annotations` as a JSON text and not as a JSON object.


### Messages and errors

| Message | Where | Meaning and action |
| --- | --- | --- |
| Applications imported successfully | Notification | The task finished. |
| Error importing applications | Notification | The portal could not start the task. Try again. |
| `Failed to verify the integrity: Hash of voters file: ... does not match with the input hash: ...` | Task log | The hash you typed is not the hash of the file. (The message says "voters file" for this file too.) |
| `Error reading CSV record` | Task log | A row has the wrong number of columns or wrong quotes. |
| `Error inserting application: ...` | Task log | The database refused a row. Check `election_event_id` and `area_id`. |
| `document not found` | Task log | The upload failed. Upload the file again. |

## Related pages

- [Approvals](../02-election-event/14-election_management_election-event_approvals.md): the **Approvals** tab, **Import** and **Export**.
- [Voter File](03-voters.md): the voters that the applications match.
- [Import File Formats](index.md): all import files.
