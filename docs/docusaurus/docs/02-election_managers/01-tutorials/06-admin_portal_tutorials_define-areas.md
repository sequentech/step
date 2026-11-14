---
id: admin_portal_tutorials_define-areas
title: Define Areas
description: "Areas allow you to organize an election into specific geographic or logical divisions, such as wards or districts. This structure enables you to assign specific contests to relevant groups of voters."
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

import GoogleVideo from '@site/src/components/GoogleVideo';

<GoogleVideo id="1mt0vOfcXRwW0cviJMwzawJWe6hC9Mf0L" />

Areas allow you to organize an election into specific geographic or logical divisions, such as wards or districts. This structure enables you to assign specific contests to relevant groups of voters.

## Accessing the Areas Menu

To manage areas, open the election event and click the **Areas** tab.

![Areas Menu Location](./assets/areas_menu_navigation.png)

From this screen, you can view existing areas, the contests assigned to them, and perform management actions like editing or deleting.

## Creating a New Area

Follow these steps to define a new area within your election:

1. Click **Add**. If the list is empty, click **Create Area**.
2. **Name:** Enter a unique name for the area (e.g., "Ward 4").
3. **Description:** Provide an optional description.
4. **Contests:** Select one or more contests that should be available to voters in this specific area. Type three or more letters of the contest name to find it.

![Area Configuration Dialog](./assets/area_config_details.png)

5. **Parent:** If the area is part of a larger hierarchy, you can select a parent area.
6. **Allow Early Voting:** Turn it on if the voters of the area can vote early.
7. Click the save button.

## Managing and Importing Areas

The Areas interface provides several tools for efficient organization and bulk data management.

* **Search:** Quickly find an area by typing its name or description into the search bar.
* **Filters:** Use the filter button to narrow down the area list by specific criteria.
* **Import:** Click **Import** to upload areas in bulk from a CSV file without a header row. Each row has six columns: identifier, country code, code, area name, delete flag and early voting policy. Only rows with `0` in the delete flag are imported.
* **Upsert Areas:** Upload a CSV file with a header row to set the parent of existing areas. Column 1 is the area name and column 2 is the name of the parent area.

:::caution
An import links each imported area to **all** the contests of the election event. After the import, open each area and remove the contests that are not for that area.
:::

:::info
**Integrity Check:** When importing area data, the system allows you to paste a **SHA-256 hash** to verify the file's authenticity and ensure it hasn't been tampered with.
:::

![Import Areas Dialog](./assets/import_areas.png)

1. Drop your CSV file in the upload area.
2. Click **Import** to finalize the process.
