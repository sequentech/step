---
id: hardening
title: Configuring the Deployment
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This chapter explains the configuration that the installer applies. **You do
not run a command for each heading below.** Supply the values in `site.json`,
then run the single installation command from a root terminal in the extracted
`deployment` directory on the administration workstation:

```bash
./deploy install site.json
```

Run it only after `./deploy check site.json` has passed, as shown in
[Install from one configuration file](01-about.md#install-from-one-configuration-file).
If installation is already running, leave that terminal open instead of starting
another copy. C1 and C2 are the two servers; the installer connects to them using
`nodes` and `ssh_identity` from your configuration.

**What you will see.** The terminal reaches
`INSTALL 2/4: harden configuration and create private service credentials`,
then `INSTALL 3/4: verify running platform configuration`. The individual
server-side command output is captured by the installer, so you do not need to
look for the old per-operation output or type additional server commands.

![Actual VM installation terminal showing the automated installation stages and completed platform verification](./assets/installation/installer-complete.png)

*Actual VM rehearsal capture. The installation command applies the configuration
below and then verifies the platform. Site names and image identities are
laboratory examples. This screenshot shows the workflow; your delivery identity
and every required check must still match your installation.*

**If it stops.** Keep the reported stage and error. Do not rerun an individual
transformation, delete `/root/deploy`, regenerate passwords or relax a policy.
The fresh-install command refuses an occupied or partially installed cluster.
Resolve its actual state with the operator before retrying. After a successful
run, continue with [customer and trustee commissioning](08-installing_toe.md#create-the-customers-tenant)
and [Verifying the Installation](10-verification.md).

## Set the host names

<!-- deployment-step: 07-001 -->
<!-- deployment-step: 07-002 -->

**Automatic installation stage.** The installer substitutes the voting,
administration and verifier names from `hosts` in `site.json`. They must be the
three names covered by the site certificate. Each portal uses the voting host
for the identity service, so sign-in and token issuers agree.

Confirm the public names in `site.json` before running the command. If a name is
wrong after installation has started, stop and correct the deployment through
the controlled recovery procedure; do not redirect a portal to another identity
service or edit a running container.

## Set the browser storage endpoint

<!-- deployment-step: 07-003 -->

**Automatic installation stage.** The public storage endpoint is the voting
portal's HTTPS origin with no path suffix. The browser allowlist contains only
the three portal HTTPS origins. The private endpoint stays inside the cluster.
The ingress preserves each signed request's host, bucket path and query; it
keeps the existing `/storage` route for public assets and does not expose the
storage administration console. Private objects still require valid signed URLs.

A failed signed upload is not corrected by stripping `/storage`, a bucket name
or query parameters from its URL. Use the storage and cross-origin checks in
[Verifying the Installation](10-verification.md).

## Set the displayed software identity

<!-- deployment-step: 07-004 -->

**Automatic installation stage.** The installer writes `APP_VERSION` and
`APP_HASH` from `release.version` and `release.step_commit` in `site.json`, checked
against the accepted delivery. For this TOE the version is `10.0.0`; the hash is
the exact 40-character Step revision that produced the accepted images.

Check the visible header using [Version](10-verification.md#version). Do not
choose another version or hash merely to match a screen or example.

## Generate the deployment secrets

<!-- deployment-step: 07-005 -->

**Automatic, one-time installation stage.** The installer generates the service
passwords and client secrets and writes `/root/deploy/secrets.env` on C1 with
root ownership and mode `600`. It replaces demonstration credentials before
services start. The object-storage client secret matches the storage root
password. `operator.service_administrator` supplies the service administrator's
name; it is not a personal login account.

Never print or copy this file into a terminal transcript. Never regenerate it
after first startup: the databases and encrypted application data depend on the
original values. An existing file during fresh installation requires investigation,
not replacement.

## Create the service credentials

<!-- deployment-step: 07-006 -->
<!-- deployment-step: 07-007 -->

**Automatic installation stage.** The installer creates
`keycloak-bootstrap-admin` and `keycloak-service-credentials` as encrypted cluster
Secrets. Harvest and both Windmill services receive the service client identity
and secret through Secret references. People must not use these credentials.

The database, storage and identity-service credentials are part of the protected
backup. Recovery must retain those original values; an `AlreadyExists` error does
not authorize deleting or replacing a Secret.

## Run the identity service in production mode

<!-- deployment-step: 07-008 -->
<!-- deployment-step: 07-009 -->

**Automatic installation stage.** The installer applies the reviewed identity
service transformation. Keycloak starts with `start --import-realm`, its fixed
public hostname, the authentication server's CA and the required outbound TLS
profile. The realm is loaded from a Secret. It does not use development mode or
accept an arbitrary hostname.

## Close the GraphQL console

<!-- deployment-step: 07-010 -->

**Automatic installation stage.** The console and development mode are disabled.
Only `/hasura/v1/graphql` is routed, with an exact-path match on each portal host.
The metadata API and console are not externally exposed. Do not expose either
for troubleshooting.

## Limit cross-origin access

<!-- deployment-step: 07-011 -->

**Automatic installation stage.** Both the ingress and the GraphQL backend allow
exactly the three configured portal HTTPS origins. Neither list contains `*`.
An ingress allowlist alone cannot remove a permissive header returned by the
backend; [GraphQL endpoint and cross-origin requests](10-verification.md#graphql-endpoint-and-cross-origin-requests)
checks the deployed behavior.

## Block the master realm

<!-- deployment-step: 07-012 -->

**Automatic installation stage.** The ingress refuses access to the identity
service's master realm on all three portal hosts. The master realm holds the
service administrator, not personal users. The external discovery checks must
receive HTTP `403`; a connection failure is not evidence of a successful block.

## Give harvest its service

<!-- deployment-step: 07-013 -->

**Automatic installation stage.** Harvest receives a cluster-internal Service on
port 8400. No public, NodePort or LoadBalancer service is substituted for it.

## Keep the results and kiosk portals off

<!-- deployment-step: 07-014 -->

**Automatic installation stage.** The results-portal and kiosk-portal addresses
remain unset; neither portal is deployed in this configuration. The realm
transformation disables the corresponding unused clients. Do not add an address
to make an unavailable screen reachable.

## Configure the realms

<!-- deployment-step: 07-015 -->
<!-- deployment-step: 07-016 -->
<!-- deployment-step: 07-017 -->
<!-- deployment-step: 07-018 -->
<!-- deployment-step: 07-019 -->

**Automatic installation stage.** The installer transforms the initial tenant
realm, the template for new tenants and the template for new election events.
The original templates are imported only at their intended creation point.
Changing a template later does not prove that an existing realm changed.

The required sign-in policy is:

- Require TLS for external requests; use a 900-second idle timeout, brute-force
  protection with initial failure factor 5, and authentication-event recording
  through `custom-event-listener`.
- Disable self-registration, password reset and "remember me". Redirect personal
  sign-in to the sole external identity provider. Disable local password and
  one-time-code forms, and link a first sign-in only to an existing account.
- Disable the local `mfa-method-selector` and its default required action.
  The external provider still enforces the user's required factors.
- Use authorization code flow with PKCE `S256` and exact portal redirect
  addresses. Disable password grants for portal clients and `admin-cli`, and
  disable `cli-account-admin`.
- Preserve the external authentication time needed for fresh board authorization:
  `passMaxAge` is `true`; the broker carries `auth_time` and `acr` to the tenant
  session. A new access token is not a substitute for a fresh external login.
- Use random client secrets and native trustee passwords. Personal accounts have
  no local password, and demonstration credentials are removed.
- Remove the board-only and prohibited named permissions from seeded
  administrator groups. Give no group `publish-results-write`, `datafix-account`
  or `voter-manually-verify`. The remaining table-permission limitations documented
  in ADV are not resolved merely by hiding portal controls or removing a named role.
- In event realms, disable voter sign-in codes, messaging channels, offline access,
  the account console and the unused channel/results clients.

The relevant permissions and values are described in
[Users, Roles and Role Separation](../../02-election_managers/04-administrator_handbook/04-users_and_roles.md)
and [Authentication and Session Settings](../../02-election_managers/04-administrator_handbook/08-authentication_and_sessions.md).
Run the deployed checks in [Verifying the Installation](10-verification.md);
do not accept only the transformed template as proof of live realm settings.

## Add the installation account

<!-- deployment-step: 07-020 -->

**Automatic installation stage.** The installer creates the personal bootstrap
operator described by `operator.username` and `operator.email`. It has no local
password and signs in through the external provider. It is used only to create
the customer's tenant. Commissioning then creates the customer's first personal
administrator and disables installation personal accounts.

## Write the documents back

<!-- deployment-step: 07-021 -->

**Automatic installation stage.** Realm documents and templates are written as
Secrets, never secret-bearing ConfigMaps. Setup jobs read the templates through
Secret references. After the first realm import, the installer removes import
credentials from the stored realm manifest; the live realm retains the values
it imported.

Do not distribute the working deployment directory. Verify platform security
with [Verifying the Installation](10-verification.md), then finish the explicit
browser, trustee-custody and handover checks.
