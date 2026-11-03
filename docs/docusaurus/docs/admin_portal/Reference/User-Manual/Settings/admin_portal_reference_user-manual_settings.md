---
id: admin_portal_reference_user_manual_settings
title: Settings
description: "The Settings page holds the configuration of the tenant. The settings apply to all the election events of the tenant. The tenant administrator sets them once, before the first election event."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Settings** page holds the configuration of the tenant. The settings apply to all the
election events of the tenant. The tenant administrator sets them once, before the first
election event. See [Set Up the Tenant](../../../procedures/01-tenant.md).

## Open the page

In the menu on the left, click **Settings**. The page has the header **Settings** and
"General Configuration".

The **Settings** menu item needs the **View settings** permission. The page also needs the
**Edit Tenant** permission. Without both, the page shows "You don't have permission to access
settings.". In version 9.0 the **View ... settings** permissions of each tab are not used: all
tabs show.

## Tabs

| Tab | Use | Reference page |
| --- | --- | --- |
| **ELECTION TYPES** | A list of election types. Version 9.0 does not use it in the elections. | [Election Types (Deprecated)](election-types-deprecated/admin_portal_reference_user-manual_settings_election-types-deprecated.md) |
| **VOTING CHANELS** | The voting channels of the tenant. The tab name has a spelling error in version 9.0. | [Voting Channels](Voting-Channels/admin_portal_reference_user-manual_settings_voting-channels.md) |
| **TEMPLATES** | Shows if email and text messages are available. Read only. | [Templates (Deprecated)](templates-deprecated/admin_portal_reference_user-manual_settings_templates-deprecated.md) |
| **LANGUAGES** | The languages that election events can use. | [Languages](Languages/admin_portal_reference_user-manual_settings_languages.md) |
| **LOCALIZATION** | Changes to the texts of the portals for all election events. | [Localization](Localization/admin_portal_reference_user-manual_settings_localization.md) |
| **Look & Feel** | The logo, the style and the help links of the tenant. | [Look & Feel](look--feel/admin_portal_reference_user-manual_settings_look--feel.md) |
| **TRUSTEES** | The trustees of the tenant. | [Trustees](Trustees/admin_portal_reference_user-manual_settings_trustees.md) |
| **Countries** | The countries from which voting and enrollment are blocked. | [Countries](countries-blocked-countries/admin_portal_reference_user-manual_settings_countries-blocked-countries.md) |
| **Backup / Restore** | The backup and the restore of the tenant configuration. | [Backup / Restore](backup--restore/admin_portal_reference_user-manual_settings_backup--restore.md) |

Version 9.0 has no **Integrations** tab and no default language selector.
