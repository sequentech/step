---
id: admin_portal_tutorials_export-data
title: Export Data
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The Admin Portal provides two voter export paths:

- **Voters > Export** creates a CSV containing the voters in the current election event.
- **Election Event > Export** creates an election-event archive and can optionally include a voters CSV.

## Secret voter fields

Secret voter fields are always omitted from standalone voter CSVs and election-event archives. Neither plaintext values nor encrypted envelopes are exported, even when the operator has secret-read permission or the archive is password protected.

The archive's S3-files option also omits secret-bearing report documents and objects whose document metadata is not yet available.
