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

Secret voter fields are omitted from exports by default. This prevents encrypted storage values from appearing in a CSV when the user did not request access to their plaintext values.

To include decrypted secret fields in a standalone voters CSV, select **Include decrypted secret voter fields**. The user starting the export must have the `voter-secret-attribute-read` permission. The export task verifies this authorization before reading the fields.

An election-event archive includes decrypted secret voter fields only when all of the following are true:

1. **Include Voters** is selected.
2. **Encrypt with Password** is checked, so the resulting archive is password protected.
3. The user starting the export has the `voter-secret-attribute-read` permission.

The **Encrypt with Password** checkbox must be selected explicitly. Choosing another export option that requires archive encryption (such as reports, applications, or bulletin-board data) does not opt in to exporting voter secrets.

If the user does not have that permission, the election-event export still includes the ordinary voter fields but omits all secret fields. Archives that contain decrypted voter secrets are marked as sensitive, so downloading them also requires the secret-read permission.

The **S3 files** option can also include secret-bearing reports when **Encrypt with Password** is
explicitly selected and the operator has secret-read permission, even if **Include Voters** is off.
Without both safeguards, these documents are excluded. Objects without committed document metadata
are always excluded. CSV secret exports remain plaintext downloads with an explicit warning; store
and transfer them securely. Importing these CSV columns still requires secret-write permission and
encrypts each value for the destination event and voter.
