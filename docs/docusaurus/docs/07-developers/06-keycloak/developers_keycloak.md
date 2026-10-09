---
id: developers_keycloak
title: Developers Keycloak
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->




This is a placeholder page for the section: Keycloak.

Content will be added here soon.

## Tenant realm bootstrap

Realm templates do not contain user passwords, OTPs or other user credentials.
Realm imports remove user credentials while preserving user metadata and configured
client-backed service accounts. New tenants retain their service-account users and
an `admin` account without a password, with the `UPDATE_PASSWORD` required action.
Other human template users, including demonstration API and trustee accounts, are
not seeded into a new tenant.

An operator must initialize each new tenant administrator through the authenticated
Keycloak administration console: select the new tenant realm, open the `admin`
user's Credentials tab and set a temporary password. The administrator then signs
in interactively and chooses their own password. Create actual trustee accounts
through the normal trustee workflow. Service-account client authentication keeps
its configured client secrets and roles.

The remote deployment configuration script also removes user credentials from its
prepared realm JSON while continuing to generate client secrets. Updating templates
or configuration files does not rotate existing deployed accounts. For existing
realms, replace passwords for the seeded human accounts or disable unused accounts,
terminate their sessions and inspect their login events. Coordinate changes with
actual trustees before a ceremony. Update both the mounted realm template and the
S3 object referenced by `KEYCLOAK_TENANT_REALM_CONFIG_S3_KEY`; otherwise a later
MinIO configuration run can upload an older template again. No application import
should be used as a substitute for this operator migration.

Run the focused Windmill `realm_user_credentials` and `tenant_bootstrap_admin`
regressions. The source-only configuration regression runs with
`python3 scripts/test_tenant_bootstrap_configuration.py` against an isolated tree
with synthetic accounts and credentials.
