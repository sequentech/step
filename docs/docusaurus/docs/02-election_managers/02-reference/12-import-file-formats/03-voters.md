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
**Import**. For the steps, see [Add the Voters](../../03-procedures/03-voters.md).

The same file format is used on the **Users** page of the tenant, to import administrators. In
that case, the import does not look up areas and does not read secret attributes.

## Fields

The first row of the file is the header. The header names the columns. The order of the columns
is not important. All columns are optional. The table lists the columns that have a special
meaning.

| Column | Required | Allowed values | Default | Meaning | Example |
| --- | --- | --- | --- | --- | --- |
| `username` | Optional | Text. The import changes it to lower case. | A random ID | The user name of the voter. It must be unique in the election event. | `voter001` |
| `email` | Optional | An email address. The import changes it to lower case. | Empty | The email address of the voter. | `voter001@example.com` |
| `email_verified` | Optional | `TRUE` or `FALSE`. The import changes it to lower case. | `TRUE` | Shows if the email address is verified. | `TRUE` |
| `first_name` | Optional | Text | Empty | The first name of the voter. | `Ana` |
| `last_name` | Optional | Text | Empty | The last name of the voter. | `Lopez` |
| `enabled` | Optional | `TRUE` or `FALSE`, in any case | `TRUE` | If `FALSE`, the voter cannot sign in. An empty cell is `TRUE`. | `TRUE` |
| `area_name` | Optional | The **Name** of an area of the election event. | No area | The area of the voter. The import stores the ID of the area. | `North` |
| `area-id` | Optional | The **Name** of an area. | No area | Same as `area_name`. Do not use both columns in one file. | `North` |
| `group_name` | Optional | The name of a group of the realm. | `voter` | The group of the voter. | `voter` |
| `password` | Optional | Text | No password | The password in plain text. The import stores only a hash of it. | `Example-Pass-1` |
| `hashed_password` | Optional | Base64 text | No password | A password that is already hashed. Use it with `password_salt`. | `Zm9v...` |
| `password_salt` | Required with `hashed_password` | Base64 text | None | The salt of the hashed password. | `YmFy...` |
| `num_of_iterations` | Optional | A whole number, 1 or more | `27500` | The PBKDF2 iterations of the hashed password. | `27500` |
| `vote-weight` | Optional | A whole number from 1 to 100 000 | Empty (weight 1) | The weight of the vote of the voter. | `3` |
| `authorized-election-ids` | Optional | One or more elections, separated with `\|` | Empty | The elections where the voter can vote. | `election-1\|election-2` |
| `id`, `election__<name>` | Not used | Any | None | Ignored on import. The voter export has the column `id` and one `election__` column for each election. | |
| Other columns | Optional | Text | Empty | Each other column becomes an attribute of the voter with the same name. | `dateOfBirth` |

### Area

- The import looks for the area by its name, in the election event. The name must be exactly the
  same as on the **Areas** tab.
- If the import does not find the area, it does not stop. The voter has no area. A voter without
  an area cannot be linked to the elections of an area. Check the area of each voter after the
  import.
- `area_name` and `area-id` are the same field. If the file has both, the import stops with the
  error `Duplicate column`.

### Group

The import adds each voter to the group in `group_name`. If the file has no `group_name` column,
the group is `voter`. If the realm has no group with this name, the import does not add the voter
to a group and does not show an error.

### Password

Use one of these options. If you do not use any, the voter has no password. This is the normal
case when the voters sign in with other data, or when you send the access data later.

- **Plain password.** Use the column `password`. The import makes a random salt for each voter and
  hashes the password with PBKDF2-SHA256 and 27 500 iterations. The import stores only the hash.
- **Hashed password.** Use the columns `hashed_password` and `password_salt`. Both values are
  Base64 text. The algorithm is PBKDF2-SHA256 and the hash length is 32 bytes. Add
  `num_of_iterations` if the hash uses another number of iterations. If the cell is empty, is not
  a number or is `0`, the import uses 27 500.

If the file has the columns `password` and `hashed_password`, the import uses `password`. If
`hashed_password` has no `password_salt`, the import stops with the error `Password salt empty`.

:::caution CAUTION: voters file with passwords
Do not keep a file with plain passwords. Delete the file after the import. If you can, give the
voters their access data with the **Send** option of the admin portal, and do not use the
`password` column.
:::

### Vote weight

Use `vote-weight` only if the election event has the weighted voting policy **Weighted Voting for
Voters**. If the policy is another one, the column is stored but the tally does not use it.

- The value is a whole number from 1 to 100 000. The import also accepts `+5` and `007`, and stores
  them as `5` and `7`.
- An empty cell means weight 1.
- The name of the column is exactly `vote-weight`, in lower case, with a hyphen. Names like
  `vote_weight` or `VoteWeight` make the import stop with an error.
- The sum of the weights of the voters of one contest area is limited to 1 000 000. The import
  does not stop if the file is over this limit. It writes a warning in the server log. Later, the
  tally refuses to count the voters of that area.

### Authorized elections

The column `authorized-election-ids` limits the elections where the voter can vote. Each value is
the **External ID** of an election. If an election has no External ID, use the ID of the election.
Separate the values with `|`.

- If the cell is empty and the voter has an area, the voter can vote in the elections of the area.
  If the area has no elections, the voter is authorized for all elections of the election event.
- If the cell is empty and the voter has no area, the voter is authorized for all elections of the
  election event.
- A value that does not match any election is ignored.

Write the External ID of the election. The voter token uses the External ID, or the ID if the election has none. The tally matches only the External ID, so for an election without one the tally does not match the voters' values.

### Secret attributes

An attribute can be marked as secret in the user profile of the election event. The import then
stores the value encrypted.

- The user must have the permission **Edit Secret Voter Fields** (`voter-secret-attribute-write`).
  If a secret column is in the file and the user does not have the permission, the import stops
  with the error `Importing encrypted voter attributes requires voter-secret-attribute-write`.
- One value can have at most 150 bytes. A longer value stops the import. Use `|` to give more
  than one value. Each value has the limit.
- An empty cell is not encrypted.
- These attributes cannot be secret: `area-id`, `authorized-election-ids`,
  `authorized-to-election-alias`, `dateOfBirth`, `disable-comment`, `email`, `firstName`,
  `first_name`, `lastName`, `last_name`, `permission_labels`,
  `sequent.read-only.id-card-number-validated`, `sequent.read-only.mobile-number`, `tenant-id`,
  `username`, `vote-weight` and `voted-channel`. If the user profile marks one of them as secret,
  the import stops.
- A secret attribute cannot use Keycloak value validators. Only the validator
  `person-name-prohibited-characters` is allowed.
- The import writes an entry in the electoral log with the names of the secret columns and the
  user who did the import.

## Rules

| Rule | Value |
| --- | --- |
| File type | `.csv` (separator: comma) or `.tsv` (separator: tab). The import decides from the end of the file name. Any other name is read as `.csv`. |
| Header | The first row. It is required. |
| Encoding | Use UTF-8. |
| Column names | Only the letters `a-z` and `A-Z`, the digits, `.`, `_` and `-`. Another character stops the import. |
| Reserved names | `sequent_internal_user_id` cannot be a column name. |
| Duplicate columns | Two headers that are the same after the import changes `.` and `-` to `_` and ignores case stop the import with `Duplicate column`. For example `area-id` and `area_id`, or `Email` and `email`. |
| Multiple values | Separate the values of an attribute with `\|`. |
| Quotes | If a value has a comma (or a tab in a `.tsv` file), put the value in double quotes. |
| Size | The import code does not check the file size. |
| Order of rows | Not important. |

:::note NOTE: optional columns of the voters file
The columns `password`, `hashed_password`, `password_salt`, `num_of_iterations`, `group_name`,
`email_verified` and `sequent_internal_user_id` are never stored as attributes. All other columns
that are not `username`, `email`, `first_name`, `last_name`, `enabled` and `election__...` become
attributes.
:::


## Example file

```csv
username,email,first_name,last_name,area_name,enabled,group_name,password,vote-weight,authorized-election-ids,sequent.read-only.mobile-number,dateOfBirth
voter001,voter001@example.com,Ana,Lopez,North,TRUE,voter,Example-Pass-1,1,,+10000000001,1990-01-31
voter002,voter002@example.com,Ben,Smith,South,TRUE,voter,Example-Pass-2,3,election-1|election-2,+10000000002,1985-06-15
voter003,voter003@example.com,Eva,Brown,North,FALSE,voter,,,,,
```

The first voter votes with weight 1 in all elections of the area North. The second voter votes with
weight 3 in `election-1` and `election-2`. The third voter is disabled and has no password.

## What the import does

1. The admin portal uploads the file. Then it asks you for the **Integrity Check (SHA-256)**. If
   you click **Import** with an empty field, the portal asks **Import Without Integrity Check?**.
   Click **Yes, Import without Integrity Check** or **Go Back**.
2. The portal starts the task **Import Users** and shows it in the task widget. The message
   **Voters Import Scheduled Successfully** is not a result of the import. It only confirms that
   the task started.
3. The task checks the hash. It checks the header. It checks the permission for secret columns.
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
| Voters Import Scheduled Successfully | Notification | The task started. Open the task and wait for **Success**. |
| Error importing Voters | Notification | The portal could not start the task. Try again. |
| Error uploading file | Notification | The upload failed. Select the file again. |
| File uploaded to server - but not imported yet | Notification | The file is on the server. Click **Import**. |
| `Failed to verify the integrity: Hash of voters file: ... does not match with the input hash: ...` | Task log | The SHA-256 you typed is not the hash of the file. Check the file and the hash. |
| `CSV Header contains characters not allowed: <name>` | Task log | A column name has a character that is not allowed. |
| `CSV header sequent_internal_user_id is reserved` | Task log | Rename the column. |
| `Duplicate column <name> in the import file: two headers map to the same field` | Task log | Remove the duplicate column. |
| `Column <name> is not recognised. The per-voter vote weight column is spelled exactly vote-weight ...` | Task log | Rename the column to `vote-weight`. |
| `Invalid vote-weight value "<value>" on row <n>: must be between 1 and 100000` | Task log | Correct the value in that row. The row number counts the header as row 1. |
| `Importing encrypted voter attributes requires voter-secret-attribute-write: <columns>` | Task log | You do not have the permission to write secret attributes. Ask an administrator, or remove the columns. |
| `Voter secret attribute <name> exceeds the 150-byte plaintext limit` | Task log | Shorten the value. |
| `Password salt empty` | Task log | Add the column `password_salt`. |
| `Error executing INSERT USER transaction` | Task log | The database refused a row. The message does not say which row or which rule failed. Common reasons: a user name that exists, an email address that exists, or two rows with the same email address (the import refuses them even if the realm allows duplicate emails), or a value in `enabled` or `email_verified` that is not `TRUE` or `FALSE`. |


## Related pages

- [Add the Voters](../../03-procedures/03-voters.md): the steps to import the file.
- [Voters](../02-election-event/05-election_management_election-event_voters.md): the **Voters** tab.
- [Area File](04-areas.md): the areas must exist before the voter import.
- [Import File Formats](index.md): all import files.
