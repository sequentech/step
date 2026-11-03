---
id: election_management_election_event_areas
title: Areas
description: "The Areas tab holds the areas of the election event. An area is a group of voters. The contests of an area are the contests that its voters see on the ballot."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Areas** tab holds the areas of the election event. An area is a group of voters. The
contests of an area are the contests that its voters see on the ballot.

## Open the tab

1. In the menu on the left, click the election event.
2. Click the **Areas** tab.

The tab shows with the **View Election Event Areas** permission. It is hidden when the election
event is locked down.

## Toolbar

| Button | Use | Permission |
| --- | --- | --- |
| **Columns** | Selects the columns of the list. | **Election Event Areas Columns** |
| **Add filter** | Filters the list by **Name**, **Description**, **ID** or **Type**. | **Election Event Areas Filters** |
| **Add** | Opens the form to create an area. | **Create Area** |
| **Import** | Opens the **Import Areas** panel. | **Import Area** |
| **Upsert Areas** | Opens a panel to set the parent of existing areas from a file. | **Upsert Area** |

When the list is empty, the tab shows "No Areas yet." with the buttons **Create Area** and
**Import**.

## Columns

| Column | Content |
| --- | --- |
| **Id** | The identifier of the area. |
| **Name** | The name of the area. Voter files refer to the area by this name. |
| **Description** | The description of the area. |
| **Contests** | The contests of the area. |
| **Actions** | Edit and delete icons. |

## Area form

| Field | Use |
| --- | --- |
| **Name** | The name of the area. |
| **Description** | The description of the area. |
| **Contests** | The contests of the area. Type three or more letters of a contest name, then select it. |
| **Parent** | The larger area that contains this area, if any. Type three or more letters to search. |
| **Save** | Saves the area. |

## Import areas

The import file is a CSV file without a header row. Each row has five columns:

| Position | Content |
| --- | --- |
| 1 | The name of the area. It becomes the **Name**. |
| 2 | A country code. The import does not use it. |
| 3 | A code. The import does not use it. |
| 4 | The description. It becomes the **Description**. |
| 5 | The delete flag. Only rows with `0` are imported. |

1. Click **Import**.
2. Type the SHA-256 hash of the file in **Integrity Check (SHA-256)**.
3. Upload the file.
4. Click **Import**.

**Expected result:** the message "Areas Imported Successfully" shows.

:::caution CAUTION
The import links each new area to all the contests of the election event. After the import,
open each area and remove the contests that are not for that area.
:::

## Upsert areas

**Upsert Areas** reads a CSV file with a header row and two columns: the name of an existing
area and the name of its parent area. It changes only the parent of the areas.

## If there is a problem

| Problem | Action |
| --- | --- |
| "Could not create Area" | Check the fields, then save again. |
| "Error importing Areas" | Check that the file has five columns and no header row, then import again. |
| "Error deleting area" | The area could not be deleted. Contact Sequent support if the error stays. |
| A contest is not in the **Contests** list. | Type three or more letters of the contest name. |

## Related pages

- [Create the Election Event](../../../../../procedures/02-event.md#6-create-the-areas)
- [Voters](../Voters/election_management_election-event_voters.md)
- [Publish](../Publish/election_management_election-event_publish.md)
