---
id: harvest_token_verification
title: Harvest token verification
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Harvest trusts a request's bearer token only after verifying it against
`certs.json`, the JWKS in the public bucket that Hasura also uses
(`HASURA_GRAPHQL_JWT_SECRET`'s `jwk_url`). There are no per-request calls to
Keycloak.

## What is checked

1. The token header's `kid` names a key of `certs.json`.
2. The signature, with the algorithm of that key, and `exp`.
3. The key's `realm` is the realm of the token's `iss`
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

Windmill tags every key it writes to `certs.json` with the realm it comes from,
in a `realm` member. JWKS consumers ignore unknown members, so Hasura reads the
file as before. Harvest rejects tokens signed with a key that has no `realm`.

Keys written by earlier releases have no `realm`. Beat runs `tag_jwks_realms`
when it starts and then every `AWS_S3_JWKS_TAG_INTERVAL_SECS` (default 3600).
Windmill then asks Keycloak for each realm's keys and tags those it finds.
Keys that no realm has are left untagged, and Harvest does not trust them.

## Cache

Each Harvest process keeps `certs.json` for the `max-age` of its
`Cache-Control` header, which Windmill sets from `AWS_S3_JWKS_CACHE_POLICY`. A
token signed with a key that is not cached, or that has no `realm` yet, makes it
read the file again. That happens at most once every
`AWS_S3_JWKS_MIN_REFRESH_SECS` (default 10), so new realms' keys are used right
away. If the file cannot be read, Harvest keeps the keys it has and retries
after that interval.

| Variable | Service | Default |
| --- | --- | --- |
| `AWS_S3_JWKS_MIN_REFRESH_SECS` | Harvest | 10 |
| `AWS_S3_JWKS_FETCH_TIMEOUT_SECS` | Harvest, Windmill | 5 |
| `AWS_S3_JWKS_TAG_INTERVAL_SECS` | Beat | 3600 |
| `AWS_S3_JWKS_CERTS_PATH` | Harvest, Windmill | `certs.json` |
