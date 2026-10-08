---
id: admin_portal_reference_user_manual_settings_languages
title: Languages
description: "The LANGUAGES tab sets the languages that the election events of the tenant can use. The tab says \"Enable languages in the system. Only languages enabled here will be available for election events.\""
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **LANGUAGES** tab sets the languages that the election events of the tenant can use. The
tab says "Enable languages in the system. Only languages enabled here will be available for
election events."

## Open the tab

1. In the menu on the left, click **Settings**.
2. Click the **LANGUAGES** tab.

## Fields

The tab has one switch for each language of the platform. By default only English is on.

1. Click the switch of a language to turn it on or off.

**Expected result:** the admin portal saves the change immediately. The language is available
in the **Language** section of the **Data** tab of the election events. The languages that are
on also show in the language menu at the top of the admin portal.

Version 9.0 has no default language selector on this tab. Each election event sets its
**Default** language on its **Data** tab.

## If there is a problem

| Problem | Action |
| --- | --- |
| A language is not in the list. | The platform has no translation for that language. Contact Sequent support. |
| A language is not in the **Language** section of an election event. | Turn on the language here. |

## Related pages

- [Settings](../admin_portal_reference_user-manual_settings.md)
- [Localization](../Localization/admin_portal_reference_user-manual_settings_localization.md)
- [Election event: Data](../../Election-Management/Election-Event/Data/election_management_election-event_data.md)
