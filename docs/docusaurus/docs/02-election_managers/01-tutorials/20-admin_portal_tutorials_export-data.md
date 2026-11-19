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
Scanovate authenticator's `liveness-secret`.

To carry the real secret, select **Integration secrets (service credentials)**
in the export dialog. The option is shown only to users with the
`integration-secret-read` permission and always encrypts the archive with a
password. The server checks the permission again, reads the secret from
Keycloak's own storage together with the configuration's `liveness-url`, and
refuses the export if a Scanovate configuration has no usable secret or changed
while it was exported. Only the Scanovate `liveness-secret` travels; other
authenticator, identity-provider and client secrets never do. Downloading the
archive or its password also requires `integration-secret-read`, and such
archives are never included in the **S3 Files** of later exports.

Anyone holding the archive and its password holds the secret. Importing the
archive installs the realm configuration as it is, including the secret and its
`liveness-url`, except the settings still masked (see
[Other Keycloak realm secrets](#other-keycloak-realm-secrets)); Liveness Plus must
use the same secret for the imported event to work.

## Other Keycloak realm secrets

Keycloak exports `**********` in place of most secrets of the event's Keycloak realm, so an
election-event archive does not contain them. When the archive is imported, settings that hold
`**********` are left unset instead of being saved as `**********`, and the import task's log lists
each one in a line that names the setting, never its value. Re-enter them in the Keycloak admin
console of the imported event's realm:

| Setting | Where to re-enter it |
|---------|----------------------|
| SMTP server password or token client secret | **Realm settings > Email** |
| Identity provider client secret | **Identity providers >** the provider |
| Client private keys, such as a SAML signing key (Keycloak 26.8 and later) | **Clients >** the client |
| Component secrets, such as an LDAP bind credential | **User federation >** the provider |
| Authenticator settings | **Authentication >** the flow **>** the step's settings |

Keycloak masks every authenticator setting that no installed authenticator declares as non-secret,
even when it is not secret, so check the import's log after each import. The Scanovate
`liveness-secret` is the exception: an import refuses a masked one instead (see
[Integration secrets](#integration-secrets)).

Client secrets are not listed: the import sets the platform clients' secrets from the deployment
and Keycloak generates new ones for the other clients. It also drops the clients' rotated secrets
and registration access tokens. The `digital-certificates` identity provider gets a new secret
shared with its client. Vault references (`${vault.…}`) are not secrets in the export and are
imported as they are.
