<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Release NEXT

## ✨ Remove tagalo from admin settings in janitor

In order to ensure that tagalo is not active as a language in the admin portal, ensure
that in the excel file you're using for janitor, you have this configuration: in the
`Parameters` tab, add a row with:

- type: admin
- key: tenant_configurations.settings.language_conf.enabled_language_codes
- value: ["en"]