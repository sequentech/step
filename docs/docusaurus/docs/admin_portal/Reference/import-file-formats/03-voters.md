---
id: import_format_voters
title: Voter File
sidebar_position: 3
description: "The format of the voter file: the columns, the rules, an example file and what the import does with each row."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The voter file is a text file with one row for each voter. You make it yourself, for example from
a voter list or a spreadsheet. You import it in an election event, on the **Voters** tab, with
**Import**. For the steps, see [Add the Voters](../../procedures/03-voters.md).

The same file format is used on the **Users** page of the tenant, to import administrators. In
that case, the import does not look up areas. The value of `area_name` is stored as it is written.

## Fields

The first row of the file is the header. The header names the columns. The order of the columns
is not important. All columns are optional. The table lists the columns that have a special
meaning.

| Column | Required | Allowed values | Default | Meaning | Example |
| --- | --- | --- | --- | --- | --- |
| `username` | Optional | Text. The import changes it to lower case. | A random ID | The user name of the voter. It must be unique in the election event. | `voter001` |
| `email` | Optional | Text. The import changes it to lower case. The import does not check the format. | Empty | The email address of the voter. | `voter001@example.com` |
| `email_verified` | Optional | `TRUE` or `FALSE`. The import changes it to lower case. An empty cell stops the import. | `TRUE` if the column is not in the file | Shows if the email address is verified. | `TRUE` |
| `first_name` | Optional | Text | Empty | The first name of the voter. | `Ana` |
| `last_name` | Optional | Text | Empty | The last name of the voter. | `Lopez` |
| `enabled` | Optional | `TRUE` or `FALSE`, in any case. An empty cell stops the import. | `TRUE` if the column is not in the file | If `FALSE`, the voter cannot sign in. | `TRUE` |
| `area_name` | Optional | The **Name** of an area of the election event. | No area | The area of the voter. The import stores the ID of the area. | `North` |
| `area-id` | Optional | The **Name** of an area. | No area | Same as `area_name`. Do not use both columns in one file. | `North` |
| `group_name` | Optional | The name of a group of the realm. | `voter` | The group of the voter. | `voter` |
| `password` | Optional | Text | No password | The password in plain text. The import stores only a hash of it. | `Example-Pass-1` |
| `hashed_password` | Optional | Base64 text | No password | A password that is already hashed. Use it with `password_salt`. | `Zm9v...` |
| `password_salt` | Required with `hashed_password` | Base64 text | None | The salt of the hashed password. | `YmFy...` |
| `num_of_iterations` | Optional | A whole number, 1 or more | `27500` | The PBKDF2 iterations of the hashed password. | `27500` |
| `authorized-election-ids` | Optional | The External IDs of one or more elections, separated with `\|` | Empty | The elections where the voter can vote. | `election-1\|election-2` |
| `id`, `election__<name>` | Not used | Any | None | Ignored on import. The voter export has the column `id` and one `election__` column for each election. | |
| Other columns | Optional | Text | Empty | Each other column becomes an attribute of the voter with the same name. | `dateOfBirth` |

### Area

- The import looks for the area by its name, in the election event. The name must be exactly the
  same as on the **Areas** tab. The search is case-sensitive.
- If the import does not find the area, it does not stop. The voter has no area. A voter without
  an area cannot be linked to the elections of an area. Check the area of each voter after the
  import.
- `area_name` and `area-id` are the same field. If the file has both, the import stops with the
  error `Error executing create table query`. Keep only one of the two columns.

### Group

The import adds each voter to the group in `group_name`. If the file has no `group_name` column,
the group is `voter`. If the realm has no group with this name, the import does not add the voter
to a group and does not show an error.

### Password

Use one of these options. If you do not use any, the voter has no password. This is the normal
case when the voters sign in with other data, or when you send the access data later.

- **Plain password.** Use the column `password`. The import makes a random salt for each voter and
  hashes the password with PBKDF2-SHA256 and 27 500 iterations. The import stores only the hash.
  Do not add the column `num_of_iterations` to this file. The import hashes the password with
  27 500 iterations but stores the value of `num_of_iterations`, so the password check can fail.
- **Hashed password.** Use the columns `hashed_password` and `password_salt`. Both values are
  Base64 text. The algorithm is PBKDF2-SHA256 and the hash length is 32 bytes. Add
  `num_of_iterations` if the hash uses another number of iterations. If the cell is empty, is not
  a number or is `0`, the import uses 27 500.

If the file has the columns `password` and `hashed_password`, the import uses `password`. If
`hashed_password` has no `password_salt`, the import stops with the error `Password salt empty`.

If the file has the column `password` and a cell in that column is empty, the import stores the
hash of an empty password for that voter. Do not leave a cell empty in this column. For voters
without a password, use a file without the column `password`.

:::caution CAUTION: voters file with passwords
Do not keep a file with plain passwords. Delete the file after the import. If you can, give the
voters their access data with the **Send** option of the admin portal, and do not use the
`password` column.
:::

### Authorized elections

The column `authorized-election-ids` limits the elections where the voter can vote. Each value is
the **External ID** of an election. If an election has no External ID, use the ID of the election.
Separate the values with `|`.

- If the cell is empty, the voter is authorized for all elections of the election event.
- A value that does not match any election is ignored.

The voter token contains the IDs of the authorized elections, not the External IDs.

## Rules

| Rule | Value |
| --- | --- |
| File type | `.csv` (separator: comma) or `.tsv` (separator: tab). The import decides from the end of the file name. Any other name is read as `.csv`. |
| Header | The first row. It is required. |
| Encoding | Use UTF-8. |
| Column names | Only the letters `a-z` and `A-Z`, the digits, `.`, `_` and `-`. Another character stops the import. |
| Duplicate columns | Two headers that have the same name after the import changes `.` and `-` to `_`, or that differ only in case, stop the import. For example `area-id` and `area_name`, or `Email` and `email`. |
| Number of fields | Each row must have the same number of fields as the header. Otherwise the import stops. |
| Multiple values | Separate the values of an attribute with `\|`. An empty cell creates no attribute. |
| Spaces | The import does not remove spaces at the start or end of a value. |
| Quotes | If a value has a comma (or a tab in a `.tsv` file), put the value in double quotes. |
| Size | The import code does not check the file size. |
| Order of rows | Not important. |

:::note NOTE: columns that are not attributes
The columns `password`, `hashed_password`, `password_salt`, `num_of_iterations`, `group_name` and
`email_verified` are not stored as attributes. The columns `id`, `username`, `email`, `first_name`,
`last_name`, `enabled` and `election__...` are not stored as attributes either. All other columns
become attributes.
:::


## Example file

```csv
username,email,first_name,last_name,area_name,enabled,group_name,authorized-election-ids,sequent.read-only.mobile-number,dateOfBirth
voter001,voter001@example.com,Ana,Lopez,North,TRUE,voter,,+10000000001,1990-01-31
voter002,voter002@example.com,Ben,Smith,South,TRUE,voter,election-1|election-2,+10000000002,1985-06-15
voter003,voter003@example.com,Eva,Brown,North,FALSE,voter,,,
```

The first voter is in the area North. The cell `authorized-election-ids` is empty, so the voter
is authorized for all elections of the election event. The second voter is in the area South and
is authorized for `election-1` and `election-2` only. The third voter is disabled.

## What the import does

1. The admin portal uploads the file when you select it. The notification **File uploaded to
   server - but not imported yet** shows when the upload is done. Then enter the **Integrity
   Check (SHA-256)** and click **Import**. If the field is empty, the portal asks **Import Without
   Integrity Check?**. Click **Yes, Import without Integrity Check** or **Go Back**.
2. The server checks that you can create voters (on the **Voters** tab) or users (on the **Users**
   page). The portal starts the task **Import Users** and shows it in the task widget.
3. If you entered a SHA-256 value, the task checks the hash. The task checks the header. It reads
   the rows.
4. The task creates all voters in one database transaction. If one row is wrong, the task makes
   no change. The task status is **Failed** and the log of the task has the reason. Correct the
   file and import it again.
5. After a good import, the voters show on the **Voters** tab. The import does not send emails or
   messages.

If the file has at least one column that becomes an attribute (for example `area_name`), the import
also gives each voter the attribute `tenant-id`.

A file without such a column gives the voters no `tenant-id` attribute.

### Messages and errors

| Message | Where | Meaning and action |
| --- | --- | --- |
| Error importing Voters | Notification | The server returned an error when the portal started the task. Try again. |
| Error uploading file | Notification | The upload failed. Select the file again. |
| File uploaded to server - but not imported yet | Notification | The file is on the server. Click **Import**. |
| `Failed to verify the integrity: Hash of voters file: ... does not match with the input hash: ...` | Task log | The SHA-256 you typed is not the hash of the file. Check the file and the hash. |
| `CSV Header contains characters not allowed: <name>` | Task log | A column name has a character that is not allowed. |
| `Error reading CSV record: <error>` | Task log | A row has a different number of fields than the header, or a row cannot be read as UTF-8. Correct the row. The message gives the position of the row. |
| `Error executing create table query: <error>` | Task log | Two headers have the same name in the import. Remove one of them. |
| `Password salt empty` | Task log | Add the column `password_salt`, with a value in each row. |
| `Error executing INSERT USER transaction: <error>` | Task log | The database refused a row. The message does not say which row or which rule failed. Common reasons: a user name that exists, or an empty cell in `enabled` or `email_verified`. |


## Related pages

- [Add the Voters](../../procedures/03-voters.md): the steps to import the file.
- [Voters](../User-Manual/Election-Management/Election-Event/Voters/election_management_election-event_voters.md): the **Voters** tab.
- [Area File](04-areas.md): the areas must exist before the voter import.
- [Import File Formats](index.md): all import files.
