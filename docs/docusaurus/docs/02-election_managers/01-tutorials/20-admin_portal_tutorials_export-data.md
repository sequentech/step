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

## Integration secrets

Keycloak masks the secret settings of authenticators, so an ordinary
election-event archive carries a placeholder (`**********`) instead of the
Scanovate authenticator's `liveness-secret`. Importing such an archive does not
install the placeholder: the setting is left out and the import task's log says
which configuration needs its secret set again.

To carry the real secret, select **Integration secrets (service credentials)**
in the export dialog. The option is shown only to users with the
`integration-secret-read` permission and always encrypts the archive with a
password. The server checks the permission again, reads the secret from
Keycloak's own storage, and refuses the export if a Scanovate configuration has
no usable secret or changed while it was exported. Only the Scanovate
`liveness-secret` travels; other authenticator, identity-provider and client
secrets never do. Downloading the archive also requires
`integration-secret-read`, and such archives are never included in the
**S3 Files** of later exports.

The archive records, without the value, which configuration, provider
(`scanovate-authenticator`) and `liveness-url` each secret belongs to. An import
installs the secret only when:

1. the importing user has the `integration-secret-write` permission;
2. the configuration is used only by Scanovate executions; and
3. its `liveness-url` and provider still match what the archive recorded.

Otherwise the secret is left out and the task log explains why. Archives without
these records (older exports, or bundles written by hand or from templates) keep
the secret they carry only for users with `integration-secret-write`. The
records guard against accidental mismatches, not against someone who can edit a
decrypted archive: anyone holding the archive and its password holds the secret.
The import never uses the destination's own secrets. Liveness Plus must be
configured with the same secret for the imported event to work.
