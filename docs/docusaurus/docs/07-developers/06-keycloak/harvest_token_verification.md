---
id: harvest_token_verification
title: Harvest token verification
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Harvest trusts a request's bearer token only after verifying it against
`certs.json`, the JWKS that Windmill writes to the public bucket and Hasura
verifies tokens with. Harvest reads it from `AWS_S3_PRIVATE_URI`,
`AWS_S3_PUBLIC_BUCKET` and `AWS_S3_JWKS_CERTS_PATH`, as Windmill does. There are
no per-request calls to Keycloak.

## What is checked

1. The token header's `kid` names a key of `certs.json`.
2. The signature, with the algorithm of that key, and `exp`.
3. The token's `aud` includes an audience of `HARVEST_JWT_AUDIENCES`. See
   [Audience](#audience).
4. The key's realm is the realm of the token's `iss`
   (`<keycloak>/realms/<realm>`).
5. The token's tenant and election-event claims are those of that realm. An
   admin token comes from `tenant-<tenant_id>`. A voter token comes from
   `tenant-<tenant_id>-event-<election_event_id>`, and when it carries an
   election-event claim, that must be its realm's event. A tenant realm's
   token with an election-event claim is refused.

Any failure answers 401. The routes then authorize the request against these
claims as before.

`decode_jwt` still reads claims without verifying them. Use it only for tokens
the process requested from Keycloak itself, as the Datafix guard does.

## Audience

A key of `certs.json` signs every token of its realm, whichever client the
token was issued to. Harvest only accepts tokens issued for it: their `aud`
claim must include one of the audiences in `HARVEST_JWT_AUDIENCES`, a
comma-separated list such as `harvest`. A token whose `aud` is missing, empty,
malformed or only names other audiences is refused with 401. The `azp` claim,
which names the client that asked for the token, is never used in its place.

`HARVEST_JWT_AUDIENCES` must name an audience that only tokens meant for Harvest
carry. If it is unset, blank, or includes `account`, `realm-management` or
`broker`, which Keycloak adds to tokens of any client, Harvest refuses every
bearer token and logs an error when it starts.

### Keycloak audience mapper

Keycloak does not put Harvest in a token's `aud` by itself. Every client whose
tokens reach Harvest, directly or through a Hasura action, which forwards the
caller's `Authorization` header, needs an **Audience** mapper
(`oidc-audience-mapper`) with:

- **Included Custom Audience**: the value of `HARVEST_JWT_AUDIENCES`, for
  example `harvest`.
- **Add to access token**: on. **Add to ID token**: off.

The mapper can go in each client's dedicated scope, or in one client scope
assigned as a default scope to those clients. In the realms Step creates, these
clients send tokens to Harvest:

| Realm | Client | Used by |
| --- | --- | --- |
| `tenant-<tenant_id>` | `admin-portal` | Admin Portal, and step-cli (`KEYCLOAK_CLI_CLIENT_ID`) |
| `tenant-<tenant_id>` | `service-account` (`KEYCLOAK_CLIENT_ID`) | The voter enrollment Keycloak extension, for `/verify-application` |
| `tenant-<tenant_id>-event-<election_event_id>` | `voting-portal` | Voting Portal |
| `tenant-<tenant_id>-event-<election_event_id>` | `voting-portal-kiosk`, `onsite-voting-portal` | Kiosk voting |
| `tenant-<tenant_id>-event-<election_event_id>` | `ivr-voting` | Telephone voting |

These are the clients Harvest's voter routes accept, plus the admin and service
clients. Add the mapper to any other client whose tokens call Harvest, such as
an SSO client that signs voters in. The Datafix guard checks the tokens it
requests itself, and Windmill does not call Harvest, so neither depends on this
check.

Hasura does not check `aud`, so the extra audience does not change how Hasura
treats these tokens.

To check a client, open **Client scopes > Evaluate** in the realm, pick a user
and the client, and look at **Generated access token**: its `aud` must include
the configured audience.

### Rollout

1. Add the audience mapper to the clients above in every tenant and election
   event realm, and to the realm templates that new realms are created from
   (`KEYCLOAK_TENANT_REALM_CONFIG_S3_KEY` and
   `KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_S3_KEY`).
2. Wait until the access tokens issued before the change have expired, or ask
   users to sign in again.
3. Set `HARVEST_JWT_AUDIENCES` and deploy Harvest.

Until every caller's tokens carry the audience, Harvest refuses those callers'
requests with 401. To roll back, deploy the previous Harvest; the mappers can
stay.

## Realm of each key

Windmill tags each key it adds to `certs.json` with its realm, in a `realm`
member. JWKS consumers ignore unknown members, so Hasura reads the file as
before. Keys added by earlier releases have no `realm`. For those, Harvest takes
the realm from the key's certificate, which Keycloak issues with the subject
`CN=<realm>` when it generates the key. Realm imports drop key material, so
Keycloak generates the keys of new realms.

## Cache

Each Harvest process keeps `certs.json` for the `max-age` of its
`Cache-Control` header, which Windmill sets from `AWS_S3_JWKS_CACHE_POLICY`.
Requests with a cached key never wait for the file. Once it expires, one
request refreshes it in the background. A token signed with a key that is not
cached waits for a refresh, so new realms' keys are used right away. Refreshes
happen at most once every `AWS_S3_JWKS_MIN_REFRESH_SECS`, so a key added less
than that after the last refresh is refused until the next one. If the file
cannot be read, or has no keys, Harvest keeps the keys it has, so emptying it
takes effect only when Harvest restarts. After a restart, Harvest refuses every
token until it can read the file.

| Variable | Service | Default |
| --- | --- | --- |
| `HARVEST_JWT_AUDIENCES` | Harvest | none: every token is refused |
| `AWS_S3_JWKS_MIN_REFRESH_SECS` | Harvest | 10 |
| `AWS_S3_JWKS_FETCH_TIMEOUT_SECS` | Harvest, Windmill | 5 |
| `AWS_S3_JWKS_CERTS_PATH` | Harvest, Windmill | `certs.json` |
