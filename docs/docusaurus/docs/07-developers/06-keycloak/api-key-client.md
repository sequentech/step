---
id: api-key-client
title: Realm defaults and API integration opt-in
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Realm defaults and API integration opt-in

New tenant realms create `api-key-client` disabled. New election-event realms
leave it absent when their template has no such client. If a fresh import contains
it, the runtime sets `enabled: false`, even if a legacy file says `true` or omits
the field. An exported enabled flag is not permission to activate an integration
in a different realm. The client is not deleted and its authentication settings
are not weakened.

Bundled tenant JSON and Janitor bundles also disable the client and omit its
sample secret so Keycloak generates a credential on direct import. Runtime
provisioning continues to sanitize client secrets before creation: Keycloak
generates unconfigured credentials, while explicitly configured internal
clients retain the configured secret delivery path. Supply secure random
credentials through that path; do not use sample values. Beyond's corresponding
chart/bootstrap template changes must ship with this Step change.

## Administrative action logging

New default tenant and election-event realms record admin actions with
`adminEventsEnabled: true`. Request representations/details remain off
(`adminEventsDetailsEnabled: false`) because they can include credential or
personal-data payloads. Ordinary login-event configuration (`eventsEnabled`,
listeners, expiry and event types) is unchanged.

Runtime creation enforces these defaults even for older imports. Generic updates
to an existing realm omit both admin-event fields so its current audit settings
survive; there is no automatic migration. To change an existing realm, make a
separate authorized choice in Keycloak's Events configuration. Enable saving
admin events while leaving **Include representation** off. Decide retention,
review access and storage capacity for the installation, then verify a synthetic
admin action is recorded without a request-body representation. Do not enable
request details as a troubleshooting shortcut.

## Enable an integration deliberately

After provisioning the intended realm:

1. Inventory the integration's client, authentication flow, realm, permissions
   and secret consumer. Confirm that it really needs `api-key-client` rather than
   the dedicated voting, admin, IVR or internal service client.
2. In Keycloak administration, select that realm and **Clients > api-key-client**.
   Verify the generated credential and deliver it through the integration's
   approved secret store. Do not copy a secret from a template or print it in
   logs. If creating the client manually in an existing realm, create it disabled
   and let Keycloak generate its secret first.
3. Enable that specific client and save. This is the explicit opt-in; no realm
   import field or deployment-wide flag is treated as opt-in. Do not broaden
   roles, enable additional grant types or disable authentication protections.
4. Verify authentication and a representative authorized integration operation,
   plus rejection of unauthorized operations. Record the dependency and operator
   decision. Keep the enabled state and credential when subsequently importing
   unrelated realm configuration.

To withdraw opt-in, disable that client explicitly after coordinating with its
consumers. Disabling new token issuance is not a promise that previously issued
JWTs are revoked; session/token revocation is a separate operator decision.

## Existing realms and migration

Generic realm upsert distinguishes a missing realm (HTTP 404) from lookup errors.
A forbidden or failed lookup stops the operation. For an existing realm, the
runtime checks whether `api-key-client` already exists. If it does, the imported
client and its service-account user are omitted from the update payload: an
import cannot replace enablement, secret or integration configuration with
exported/template values. The current secret is neither fetched nor written
back. If the client is new to the realm, the prepared payload disables it.
Other clients retain their existing import behavior.

Existing installations are not automatically migrated. Inventory callers and
coordinate a separate migration per realm; preserve enabled integrations until
an operator chooses to change them. Record current enablement and secret-store
references without exporting credential values. Test the proposed choice in an
isolated environment, plan a maintenance window if needed, and validate voting,
IVR, administration, trustee ceremonies/tallies and Datafix before rollout.
Rollback restores the operator's recorded enablement and integration settings;
rotation, revocation and restoring a changed credential require their own
coordinated procedure. Reimporting a template is not a migration or rollback tool.

## Compatibility checks

Source defaults use separate clients for ordinary portal login and internal
service access. That does not prove that every deployed consumer is independent
of `api-key-client`. Check actual configuration and successful operations.

Datafix's inbound request guard accepts the caller's client ID and secret and
obtains a tenant token, then its routes require the Datafix permission. Therefore
its credentials can refer to `api-key-client`; do not assume a dedicated Datafix
client in every installation. Audit outbound Datafix configuration separately.

Trustees or the administration CLI may also use `api-key-client`. Such
installations need the explicit opt-in above before the integration starts.
Never enable it for every realm to accommodate one consumer.

## Regression checks

From `packages/`, run the branch's sequent-core tests with its Keycloak/service
features enabled. The `realm_provisioning` unit tests and `api_key_client_upsert` integration test cover fresh tenant/event inputs,
legacy enabled and missing flags, generated/configured credential handling,
existing enabled/disabled clients, repeated imports, missing clients, unrelated
clients and failed lookups. They use synthetic bounded HTTP fixtures.

From the repository root, run:

```sh
python3 -m unittest discover -s scripts -p test_api_key_client_templates.py -v
```

These checks exercise direct template defaults, event-client absence, Janitor
bundles and remote template preparation where available. Before release, test
against the branch's real Keycloak distribution: default-disabled clients must
fail token issuance; the explicit opt-in must permit only the intended
integration's grants and permissions; a reimport must preserve an enabled
integration and its existing credential. Run the applicable voting, IVR, admin,
trustee and Datafix controls as well. A source or mock test does not replace those
runtime checks.

Use `--features default_features,keycloak` for the Rust tests.
