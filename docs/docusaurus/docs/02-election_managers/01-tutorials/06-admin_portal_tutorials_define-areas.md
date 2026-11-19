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

From this screen, you can view existing areas and the contests assigned to them. You can also edit or delete an area. You see only the actions that your role permits.

## Creating a New Area

Follow these steps to define a new area within your election:

1. Click **Add**. If the list is empty, click **Create Area**.
2. **Name:** Enter a unique name for the area (e.g., "Ward 4").
3. **Description:** Provide an optional description.
4. **Contests:** Type three or more letters of a contest name, then select the contest. Do this for each contest of the area.

![Area Configuration Dialog](./assets/area_config_details.png)

5. **Parent:** If the area is part of a larger hierarchy, you can select a parent area.
6. If the election event uses weighted voting for areas, set the **Weight** of the area.
7. **Allow Early Voting:** Turn it on if the voters of the area can vote early. You can change it only when the early voting channel is on in **Voting Channels Allowed** on the **Data** tab of the election event.
8. Click the save button.

## Managing and Importing Areas

The Areas interface provides several tools for efficient organization and bulk data management.

* **Search:** Quickly find an area by typing its name or description into the search bar.
* **Filters:** Use the filter button to narrow down the area list by specific criteria.
* **Import:** Click **Import** to upload areas in bulk from a CSV file without a header row. Each row has six columns: the area identifier, the country code, the code, the area name, the delete flag and the early voting policy. The identifier becomes the **Name** of the area and the area name becomes the **Description**. The country code and the code are not used. Only rows with `0` in the delete flag are imported. The early voting policy `allow_early_voting` allows early voting in the area.
* **Upsert Areas:** Click **Upsert Areas** to set the parent of areas that exist. Upload a CSV file with a header row. Column 1 is the **Name** of the area and column 2 is the **Name** of the parent area. The rows of areas that do not exist are ignored.

:::caution
An import links each imported area to **all** the contests of the election event. After the import, open each area and remove the contests that are not for that area.
:::

:::info
**Integrity Check:** When importing area data, the system allows you to paste a **SHA-256 hash** to verify the file's authenticity and ensure it hasn't been tampered with.
:::

![Import Areas Dialog](./assets/import_areas.png)

1. Type the hash in **Integrity Check (SHA-256)**. If you leave it empty, the admin portal asks "Import Without Integrity Check?".
2. Drop your CSV file in the upload area.
3. Click **Import** to finalize the process.
