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
   election-event claim, that must be its realm's event. A tenant realm's
   token with an election-event claim is refused.

Any failure answers 401. The routes then authorize the request against these
claims as before.

`decode_jwt` still reads claims without verifying them. Use it only for tokens
the process requested from Keycloak itself, as the Datafix guard does.

## Realm of each key

Windmill tags each key it adds to `certs.json` with its realm, in a `realm`
member. JWKS consumers ignore unknown members, so Hasura reads the file as
before. Keys added by earlier releases have no `realm`. For those, Harvest takes
the realm from the key's certificate, which Keycloak issues with the subject
`CN=<realm>` when it generates the key. Realm imports drop key material, so
Keycloak generates the keys of new realms.

## Upgrading

Until step#1887 (August 2025), Windmill created realms with the fixed keys of
the tenant and election-event realm templates: the RSA keys of their
`rsa-generated` and `rsa-enc-generated` providers, the secrets of their
`hmac-generated`, `hmac-generated-hs512` and `aes-generated` providers, and,
in tenant realms, the secrets of the `service-account` and `api-key-client`
clients. Realms created then keep them unless they were rotated. They are
public in this repository's history, so anyone can forge those realms' tokens,
including the action tokens Keycloak signs with the HMAC secret. Harvest
refuses tokens signed with the template RSA keys, so those realms' users get
401 from Harvest after the upgrade. Rotate their keys before upgrading.

A realm uses only its own keys when every certificate on its
`<keycloak>/realms/<realm>/protocol/openid-connect/certs` endpoint has the
subject `CN = <realm>`. With `kcadm.sh` logged in to the master realm, and
`KEYCLOAK_URL` set, this prints the keys of other subjects:

```bash
for realm in $(kcadm.sh get realms --fields realm --format csv --noquotes); do
  curl -s "$KEYCLOAK_URL/realms/$realm/protocol/openid-connect/certs" \
    | jq -r '.keys[].x5c[0]' \
    | while read -r cert; do
        subject=$(printf '%s' "$cert" | base64 -d | openssl x509 -inform DER -noout -subject)
        [ "$subject" = "subject=CN = $realm" ] || echo "$realm: $subject"
      done
done
```

The template keys have the subjects `CN = electoral-process` (tenant realms)
and `CN = tenant90505c8a23a94cdfa26b4e19f6a097d533f18502a67c48538333a58630663559`
(election-event realms). For each realm listed:

1. In Keycloak's **Realm settings → Keys → Providers**, add `rsa-generated`,
   `rsa-enc-generated`, `hmac-generated` (HS256 and HS512) and `aes-generated`
   providers with distinct names and a priority below the template's 100.
   Keycloak keeps using the template keys and lists the new RSA keys on the
   realm's `certs` endpoint.
2. Add the new RSA keys to `certs.json`, while no tenant or election event is
   being created or deleted, since Windmill rewrites the file then. Upload it
   with the `Cache-Control` of `AWS_S3_JWKS_CACHE_POLICY` and the content type
   `application/json`, for instance with
   `mc cp --attr "Cache-Control=$AWS_S3_JWKS_CACHE_POLICY;Content-Type=application/json"`;
   without that header Hasura stops refreshing the file. Wait for its
   `max-age`.
3. Delete the template's providers, the ones with priority 100, so that
   Keycloak uses the new keys. The realm's sessions, offline tokens and
   emailed action links stop working, so its users have to log in again.
4. In tenant realms, regenerate the credentials of the `service-account` and
   `api-key-client` clients, and update the services that use them, such as
   `KEYCLOAK_CLIENT_SECRET`.

Once no realm lists the template keys, remove them from `certs.json` the same
way.

## Cache

Each Harvest process keeps `certs.json` for the `max-age` of its
`Cache-Control` header, which Windmill sets from `AWS_S3_JWKS_CACHE_POLICY`.
Requests with a cached key never wait for the file. Once it expires, one
request refreshes it in the background. A token signed with a key that is not
cached waits for a refresh, so new realms' keys are used right away. Refreshes
happen at most once every `AWS_S3_JWKS_MIN_REFRESH_SECS`. If the file cannot
be read, or has no keys, Harvest keeps the keys it has, so emptying it takes
effect only when Harvest restarts. After a restart, Harvest refuses every token
until it can read the file.

| Variable | Service | Default |
| --- | --- | --- |
| `AWS_S3_JWKS_MIN_REFRESH_SECS` | Harvest | 10 |
| `AWS_S3_JWKS_FETCH_TIMEOUT_SECS` | Harvest, Windmill | 5 |
| `AWS_S3_JWKS_CERTS_PATH` | Harvest, Windmill | `certs.json` |
