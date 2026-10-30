---
id: developers_keycloak
title: Developers Keycloak
---



## Harvest request authentication

Harvest verifies bearer-token signatures, expiry and issuer before trusting
Hasura authorization claims. The issuer realm must match the token's tenant and
the election-event claim when present. Older event templates may omit that
optional claim; their keys still come from the exact issuer realm. Signing keys are fetched from `KEYCLOAK_URL`
over the configured internal connection and cached for five minutes. Requests
fail authentication when a needed signing key cannot be retrieved, including
while thirty-two key downloads are already in progress.

Harvest requires `KEYCLOAK_URL` and trusts it as an issuer base.
`KEYCLOAK_PUBLIC_URL` is optional and needed when tokens use that public URL as
their issuer; `KIOSK_KEYCLOAK_URL` is likewise optional for kiosk issuers. Deployments using additional frontend aliases must supply
their exact HTTP(S) base URLs in the comma-separated `HARVEST_JWT_ISSUER_URLS`
setting, including any `/auth` prefix. These URLs select accepted issuers;
key downloads always use the internal `KEYCLOAK_URL`. The checked-in local and
air-gapped configurations explicitly include their localhost issuer aliases.

Before restarting Harvest with this change, verify that every normal and kiosk
login issuer is configured and that Harvest can reach each realm's internal
`protocol/openid-connect/certs` endpoint. Existing Keycloak RS256 access tokens
and Datafix client-credential tokens use the same verification boundary. No
database or realm migration is required.


## Identity provider configuration

Default election-event realms do not include a demonstration SAML identity provider.
Configure trusted providers explicitly with your own signing certificate. Realm
imports remove providers that trust the bundled development certificate, together
with their mappers and automatic redirect settings. Custom providers remain available.

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

The template's `api-user` is disabled: it belongs to the admin group and nothing
signs in with it. Enable it only after giving it a password of its own. In the
devcontainer, `init-cli.sh` runs `.devcontainer/scripts/set-dev-passwords.sh`,
which gives the local `admin`, `trustee1` and `trustee2` the development passwords
that `config-cli.sh` and the dev tooling use (`TRUSTEE1_PASSWORD` and
`TRUSTEE2_PASSWORD` in `.devcontainer/.env`). Accounts that already have a password
keep it. It is for local development only.

This release predates the remote deployment configuration script. Provision
passwordless realm templates using your deployment tooling. Updating templates
or configuration files does not rotate existing deployed accounts. For existing
realms, replace passwords for the seeded human accounts or disable unused accounts,
terminate their sessions and inspect their login events. Coordinate changes with
actual trustees before a ceremony. Update both the mounted realm template and the
S3 object referenced by `KEYCLOAK_TENANT_REALM_CONFIG_S3_KEY`; otherwise a later
MinIO configuration run can upload an older template again. No application import
should be used as a substitute for this operator migration.

Run the focused Windmill `realm_user_credentials` and `tenant_bootstrap_admin`
regressions.
