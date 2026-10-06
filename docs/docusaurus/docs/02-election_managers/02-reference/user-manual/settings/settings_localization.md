---
id: settings_localization
title: Localization
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Tenant localization overrides customize Admin Portal text for each enabled language. The editor
offers **Admin portal** for a portal-specific override and **Global** for source-wide overrides.
Because tenant localization is consumed only by the Admin Portal, both choices currently reach
that portal; **Global** does not apply to public election-event portals. Enter the translation key
without a prefix and enter its new value.

Existing unprefixed overrides are shown as **Legacy (Admin portal)** and continue to affect only
the Admin Portal. Editing and saving one promotes it to the explicit scope selected in the editor.
No data migration is required.

Explicitly scoped keys are stored with a colon prefix such as `adminPortal:header.title`. The
Admin Portal composes the stored key automatically, so do not type the prefix into the key field.
