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
3. The key's realm is the realm of the token's `iss`
   (`<keycloak>/realms/<realm>`).
4. The token's tenant and election-event claims are those of that realm. An
   admin token comes from `tenant-<tenant_id>`. A voter token comes from
   `tenant-<tenant_id>-event-<election_event_id>`, and when it carries an
   election-event claim, that must be its realm's event.

Any failure answers 401. The routes then authorize the request against these
claims as before.

`decode_jwt` still reads claims without verifying them. Use it only for tokens
the process requested from Keycloak itself, as the Datafix guard does.

## Realm of each key

Windmill tags each key it adds to `certs.json` with its realm, in a `realm`
member. JWKS consumers ignore unknown members, so Hasura reads the file as
before. Keys added by earlier releases have no `realm`. For those, Harvest takes
the realm from the key's certificate, which Keycloak issues with the subject
`CN=<realm>`. Realm imports drop key material, so Keycloak generates every key
in `certs.json`.

## Cache

Each Harvest process keeps `certs.json` for the `max-age` of its
`Cache-Control` header, which Windmill sets from `AWS_S3_JWKS_CACHE_POLICY`.
Requests with a cached key never wait for the file. Once it expires, one
request refreshes it in the background. A token signed with a key that is not
cached waits for a refresh, so new realms' keys are used right away. Refreshes
happen at most once every `AWS_S3_JWKS_MIN_REFRESH_SECS`. If the file cannot
be read, Harvest keeps the keys it has.

| Variable | Service | Default |
| --- | --- | --- |
| `AWS_S3_JWKS_MIN_REFRESH_SECS` | Harvest | 10 |
| `AWS_S3_JWKS_FETCH_TIMEOUT_SECS` | Harvest, Windmill | 5 |
| `AWS_S3_JWKS_CERTS_PATH` | Harvest, Windmill | `certs.json` |
