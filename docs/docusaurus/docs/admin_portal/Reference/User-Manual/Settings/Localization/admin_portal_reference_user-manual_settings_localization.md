---
id: admin_portal_reference_user_manual_settings_localization
title: Admin Portal Reference User Manual Settings Localization
---

## Localization Overrides

To customize the display label of a user attribute, add a translation override via the Admin Portal under **Settings** > **Localization** > **Add**.

User attribute keys must use the prefix `usersAndRolesScreen.users.fields.` followed by the attribute name.

**Example:** to display the `personal_administrative_number` attribute as "PAN", add the following to the tenant's localization settings:

Key: `usersAndRolesScreen.users.fields.personal_administrative_number`
Value: `PAN`

This works for any attribute name, including custom ones not present in the default translations.

