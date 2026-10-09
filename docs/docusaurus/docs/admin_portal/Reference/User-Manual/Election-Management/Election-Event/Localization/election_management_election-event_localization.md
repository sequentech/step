---
id: election_management_election_event_localization
title: Localization
description: "The Localization tab changes the texts of the portals for one election event, in each language. Each change is a pair: the Key of a text and its new Value."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Localization** tab changes the texts of the portals for one election event, in each
language. Each change is a pair: the **Key** of a text and its new **Value**. To change a text
for all election events of the tenant, use **Settings** > **LOCALIZATION** instead.

## Open the tab

1. In the menu on the left, click the election event.
2. Click the **Localization** tab.

The tab shows with the **View Election Event Data** permission. It is hidden when the election
event is locked down. The actions need these permissions:

| Action | Permission |
| --- | --- |
| Select the language | **Election Event Localization Selector** |
| Add a text | **Create Localization** |
| Change a text | **Edit Localization** |
| Delete a text | **Delete Localization** |

## Screen elements

| Element | Use |
| --- | --- |
| **Select Language** | The language of the texts in the list. Only the languages of the election event show. |
| **Add** | Opens the form to add a text. |
| **Key** column | The key of the text. |
| **Value** column | The new text. |
| Edit icon | Changes the text of the row. |
| Delete icon | Deletes the row, after a confirmation. |

## Add a text

1. In **Select Language**, select the language.
2. Click **Add**.
3. Type the **Key**.
4. Type the **Value**.
5. Click **Save**.

**Expected result:** the message "Localization updated Successfully" shows. The row shows in
the list.

## If there is a problem

| Problem | Action |
| --- | --- |
| "No languages were set for the event" | Select the languages in the **Language** section of the **Data** tab. |
| "Localization update failed" | Check the key and the value, then try again. |
| The new text does not show in the portal. | Make sure that the key is correct and that you selected the correct language. |

## Related pages

- [Data](../Data/election_management_election-event_data.md)
- [Settings: Localization](../../../Settings/Localization/admin_portal_reference_user-manual_settings_localization.md)
