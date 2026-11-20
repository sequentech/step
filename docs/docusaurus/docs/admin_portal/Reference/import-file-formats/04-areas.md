---
id: import_format_areas
title: Area File
sidebar_position: 4
description: "The format of the area file: the five columns, an example file, the upsert of parent areas and what the import does with each row."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The area file is a text file with one row for each area. You make it yourself, for example in a
spreadsheet. You import it in an election event, on the **Areas** tab, with **Import**. For the
steps, see [Import areas (alternative)](../../procedures/02-event.md#import-areas-alternative).

Each row creates one new area. The import does not change areas that already exist. Import the
areas before the voters. The [voter file](03-voters.md) refers to areas by name.

## Fields

The file has no header row. The import reads columns 1, 4 and 5. It does not read columns 2 and 3.
The header names in the table come from the source format. The import does not read them.

| Column | Header name (not read) | Required | Allowed values | Meaning | Example |
| --- | --- | --- | --- | --- | --- |
| 1 | `EMB_ID` | Yes | Text | The **Name** of the area. The voter file refers to this name. | `North` |
| 2 | `CTRY_CODE` | No | Any | Not used. The import ignores it. Leave it empty. | |
| 3 | `EMB_CODE` | No | Any | Not used. The import ignores it. Leave it empty. | |
| 4 | `AREANAME` | No | Text | The **Description** of the area. | `North District` |
| 5 | `isdelete` | Yes | `0` | The import flag. Only `0` imports the row. Any other value, or an empty cell, skips the row. | `0` |

### Name

- The import uses column 1 as it is. It does not remove spaces around the name.
- The import does not check that column 1 is not empty. A row with an empty column 1 creates an
  area with an empty name. Check each row before you import.
- The import does not check for duplicate names. Each row creates a new area, even if the name
  exists. The voter import then chooses one of the areas with that name. The choice is not
  defined.

:::caution CAUTION: importing the areas file again
Importing the same file again creates the areas again. Check the **Areas** tab before you import
the file a second time.
:::

### Contests

An import links each new area to **all** the contests of the election event that exist at the
time of the import. Remove the contests that do not belong to an area. See [Rules](#rules).

## Example file

```csv
North,,,North District,0
South,,,South District,0
Old Area,,,Old District,1
```

The first row creates the area **North**. The second row creates the area **South**. The third
row is not imported, because column 5 is `1`.

## Rules

| Rule | Value |
| --- | --- |
| Header row | Do not add one. The import reads every row as data. A header row has `isdelete` in column 5, so the import skips it. Do not rely on this. |
| Columns | The same number of columns in each row. Use at least five columns. If a row has a different number of columns, the import stops with `Error reading CSV record`. |
| Separator | A comma. A field that contains a comma must be in double quotes. |
| Import flag | Column 5 must be exactly `0`. A space or another character skips the row. |
| Integrity check | If you type a SHA-256 value, the import compares it with the hash of the file. The comparison ignores upper and lower case. It does not ignore spaces. |
| No integrity check | If the field is empty, the portal asks you to confirm. The import then runs without a check. |
| All or nothing | The import saves all areas and links in one transaction. If an error occurs, no area is saved. |
| Permission | The user needs the **Import Area** permission to see the **Import** button. The server also checks the **Edit Area** permission. |
| File size | The upload limit of your environment. The import has no row limit. |

### Remove a contest from an area

1. On the **Areas** tab, click the edit icon of the area.
2. In **Contests**, remove the contests that do not belong to the area.
3. Click **Save**.

The save replaces all contest links of the area with the contests that remain in the field.

## What the import does

1. The portal uploads the file. The **Import** button is enabled after the upload ends.
2. Type the SHA-256 value of the file in **Integrity Check (SHA-256)**. This step is optional. If
   the field is empty and you click **Import**, the portal asks **Import Without Integrity Check?**.
   Click **Yes, Import without Integrity Check** to continue, or **Go Back** to stop.
3. The server checks the hash, if you typed one. A mismatch stops the import.
4. The server reads each row. It skips each row where column 5 is not `0`. For each other row, it
   creates one area with a new ID.
5. The server links each new area to all the contests of the election event.
6. The server saves all areas and links in one transaction. If one step fails, the server saves
   nothing.
7. The portal shows **Areas Imported Successfully** and refreshes the list. The import does not
   send any message.

The area import does not start a task. There is no task log for it. The result shows in the
portal only.

## Upsert parent areas

Use the **Upsert Areas** action on the **Areas** tab to set the parent area of areas that already
exist. This action does not create areas. Import the areas first. The action needs the **Upsert
Area** permission. The server also checks the **Edit Area** permission.

The file for this action is different from the area file:

- The first row is always the header. The import skips it, so the file must have a header row.
- Column 1 is the **Name** of the area. The import changes only areas with this name.
- Column 2 is the **Name** of the parent area. Leave it empty for an area without a parent.
- Each row must have the same number of columns as the header row.
- A row whose column 1 does not match an area name is skipped. The import does not show an error.
- If column 2 is empty, or does not match an area name, the import removes the parent of the area.
- The **Integrity Check (SHA-256)** field is not used. The import does not check the hash.

## Messages and errors

| Message | Where | Meaning and action |
| --- | --- | --- |
| Areas Imported Successfully | Notification | The import finished. Check the **Areas** tab. |
| Error importing Areas | Notification | The import failed. No area was saved. Check the file and the hash. |
| File uploaded to server - but not imported yet | Notification | The file is on the server. Click **Import**. |
| Error uploading file | Notification | The upload failed. Select the file again. |
| Import Without Integrity Check? | Dialog | You did not type a hash. Confirm only if the file is the correct file. |
| `Failed to verify the integrity: Hash of voters file: <hash of the file> does not match with the input hash: <typed hash>` | Server error | The typed hash is not the hash of the file. Check the hash and the file. The text says "voters file" in the code. The portal may not show this text. |
| `Failed to verify the integrity: <reason>` | Server error | The server could not read or hash the file. Upload the file again. |
| `document not found` | Server error | The server does not have the file. Upload the file again. |
| `Error reading CSV record` | Server error | The server could not read a row. Check that each row has the same number of columns. |

## Related pages

- [Import areas (alternative)](../../procedures/02-event.md#import-areas-alternative): the steps to import the file.
- [Voter File](03-voters.md): the voter file refers to areas by name.
- [Import File Formats](index.md): all import files.
