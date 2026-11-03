---
id: admin_portal_reference_user_manual_settings_localization
title: Localization
description: "The LOCALIZATION tab of Settings changes the texts of the portals for the whole tenant, in each language. Each change is a pair: the Key of a text and its new Value."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **LOCALIZATION** tab of **Settings** changes the texts of the portals for the whole
tenant, in each language. Each change is a pair: the **Key** of a text and its new **Value**.
To change a text for one election event only, use the **Localization** tab of the election
event.

## Open the tab

1. In the menu on the left, click **Settings**.
2. Click the **LOCALIZATION** tab.

## Add a text

1. In **Select Language**, select the language.
2. Click **Add**.
3. Type the **Key**.
4. Type the **Value**.
5. Click **Save**.

**Expected result:** the message "Localization updated Successfully" shows. The row shows in the
list. Use the edit and delete icons of the row to change or delete it.

## Example: the label of a user attribute

To change the label of a user attribute, use a key with the prefix
`usersAndRolesScreen.users.fields.` followed by the name of the attribute.

For example, to show the attribute `personal_administrative_number` as "PAN":

| **Key** | **Value** |
| --- | --- |
| `usersAndRolesScreen.users.fields.personal_administrative_number` | `PAN` |

This works for any attribute name, also for attributes that have no standard label.

## If there is a problem

| Problem | Action |
| --- | --- |
| "Localization update failed" | Check the key and the value, then try again. |
| The new text does not show. | Make sure that the key is correct and that you selected the correct language. |

## Related pages

- [Election event: Localization](../../Election-Management/Election-Event/Localization/election_management_election-event_localization.md)
- [Languages](../Languages/admin_portal_reference_user-manual_settings_languages.md)
- [Settings](../admin_portal_reference_user-manual_settings.md)
