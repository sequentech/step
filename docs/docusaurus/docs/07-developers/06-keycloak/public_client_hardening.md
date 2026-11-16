---
id: public_client_hardening
title: Public realm client hardening
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

Public clients run in the browser and cannot keep a secret. When windmill
creates or imports a tenant or election event realm (`upsert_realm` in
sequent-core), it restricts them to browser logins:

| Client | Settings applied |
|--------|------------------|
| `voting-portal`, `onsite-voting-portal`, `voting-portal-kiosk`, `results-portal`, `admin-portal` | Direct access (password) grant and implicit flow off; PKCE `S256` required; Web Origins `+` (the client's own redirect URIs); no request URIs. Post-logout redirect URIs `+` too, except on `voting-portal`, `onsite-voting-portal` and `voting-portal-kiosk`: their Finish button logs out to the event's finish URL (`redirect_finish_url`, `kiosk_redirect_finish_url`), which can be another site, so theirs are left as they are. |
| `admin-portal` | In tenant realms, also: root, base and redirect URIs pinned to `ADMIN_PORTAL_URL`, which windmill requires. |
| `account-console` | In every realm: redirect URIs limited to `/realms/<realm>/account/*`; Web Origins and post-logout redirect URIs `+`; no request URIs. |
| `cli-account-admin` | Confidential: the password grant now also needs the client secret. No browser logins and no request URIs. Gets any `admin-portal` token mapper it lacks and the `admin-portal` default role, so its tokens carry the same Hasura claims. |

The password grant skips the browser flow, including its email or SMS
one-time code, so on a public client a password alone was enough to get a
token. Wildcard Web Origins let any site call the APIs with a token it obtained,
and a wildcard `account-console` redirect URI could send a user's account
session to any site.

Confidential clients such as `ivr-voting`, `ivr-service`, `api-key-client` and
`service-account` are not changed.

## Scripts and the CLI

Scripts and `step-cli` that log in with a username and password must use the
confidential `cli-account-admin` client with its secret (Keycloak admin console
→ tenant realm → Clients → `cli-account-admin` → Credentials). The development
and deployment environment files set `KEYCLOAK_CLI_CLIENT_ID=cli-account-admin`;
`KEYCLOAK_CLI_CLIENT_SECRET` already holds that client's secret. See
[API authentication](../10-tutorials/05-api-authentication.md).

## Existing realms

Realms created before this change keep their old client settings until they
are migrated. `scripts/keycloak/harden_public_clients.py`, in the `beyond`
repository, applies the same settings through the Keycloak admin API. It only
prints what it would change unless `--apply` is given. From a `beyond`
checkout:

```bash
export KEYCLOAK_URL=https://keycloak.example.sequent.vote
export KEYCLOAK_ADMIN=admin
export KEYCLOAK_ADMIN_PASSWORD=...

# Review the changes for every tenant and election event realm
python3 scripts/keycloak/harden_public_clients.py realm --all-tenant-realms \
    --admin-portal-url https://admin.example.sequent.vote

# Apply them (or name realms one by one with --realm)
python3 scripts/keycloak/harden_public_clients.py realm --all-tenant-realms \
    --admin-portal-url https://admin.example.sequent.vote --apply
```

Before applying it to a deployment:

1. Switch every script and CLI configuration that uses the password grant on
   `admin-portal` or a voting portal client to `cli-account-admin`. Keycloak
   keeps no secret for a public client, so the script sets `cli-account-admin`'s
   secret to `KEYCLOAK_CLI_CLIENT_SECRET` when that variable is set, and
   generates one otherwise; read it from the Credentials tab afterwards. A
   secret Keycloak already stores is kept.
2. Pass the admin portal URL with `--admin-portal-url` so that existing tenant
   realms get the same `admin-portal` redirect URIs as new ones.

The script leaves a realm unchanged, reports it and exits with status 1 when one
of the browser clients above would keep the redirect URI `*`. Its Web Origins
`+` would allow no origin, so its portal could not log in. Pin that client's
redirect URIs first, or pass `--admin-portal-url` for `admin-portal`.

`template FILE...` applies the same settings to realm template files.
