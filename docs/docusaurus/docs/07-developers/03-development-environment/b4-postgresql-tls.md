---
id: b4-postgresql-tls
title: B4 PostgreSQL TLS
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

B4's HTTP service and native PostgreSQL clients share the same TLS settings.
Windmill uses the native client when it provisions boards or reads tally messages.
The service pool, direct client, database create/drop helpers and Braid demo tool
all require verified TLS by default.

| Setting | Meaning |
| --- | --- |
| `B4_PG_SSL_MODE=verify-full` | Require TLS and verify the server's certificate chain and hostname. Default when unset. |
| `B4_PG_SSL_MODE=require` | Alias of `verify-full`; it also verifies the certificate and hostname. |
| `B4_PG_SSL_MODE=disable` | Explicit plaintext mode for an isolated local development database. |
| `B4_PG_CA_PATH` | Optional PEM CA bundle inside each container. Unset or empty uses OpenSSL's system trust roots. |

`prefer`, invalid modes, unreadable CA files and a CA path combined with `disable`
are rejected. A TLS refusal, untrusted certificate or hostname mismatch fails the
connection; it never retries with plaintext. A Unix socket requires explicit
`disable`, because PostgreSQL does not negotiate TLS on Unix sockets.

## Managed PostgreSQL

Use `B4_PG_HOST` with the provider's certificate hostname, the direct PostgreSQL
port in `B4_PG_PORT`, and the existing user, password and database settings. Typed
connection parameters preserve spaces, quotes and backslashes in credentials.
For a private CA, mount the provider's CA bundle read-only into **both B4 and
every Windmill worker** that uses the board and set its container path:

```sh
B4_PG_SSL_MODE=verify-full
B4_PG_CA_PATH=/etc/b4-postgres-ca/ca.pem
```

The CA must sign the server's complete chain, and the certificate must name
`B4_PG_HOST`. A DNS alias or private IP absent from the certificate will fail.
Do not solve that failure by disabling verification. System trust is sufficient
only when the image already trusts the provider's issuing CA.

The existing `kubernetes-app` Helm chart passes `envVars`, `volumes` and
`volumeMounts` through to its Deployment. Apply equivalent values to B4 and
Windmill; the referenced ConfigMap must already contain the approved CA bundle:

```yaml
envVars:
  B4_PG_SSL_MODE: verify-full
  B4_PG_CA_PATH: /etc/b4-postgres-ca/ca.pem
volumes:
  - name: b4-postgres-ca
    configMap:
      name: b4-postgres-ca
volumeMounts:
  - name: b4-postgres-ca
    mountPath: /etc/b4-postgres-ca
    readOnly: true
```

Compose passes the same environment settings. A custom CA additionally needs a
read-only bind mount in each consuming service's Compose override. The development
env example explicitly selects `disable` for its bundled non-TLS PostgreSQL.
Existing installations with non-TLS databases must explicitly select that mode
or configure a verified TLS endpoint before upgrading. No certificate, credential,
database schema or live deployment change is performed by the source upgrade.

## Sessions and trustees

Use a direct database endpoint or session-mode PgBouncer. B4 retains its existing
pool and transaction boundaries, including transaction-scoped advisory locks.
The TLS connector does not change their lifetime. A transaction-mode proxy is not
a substitute for a stable session where callers retain session state or locks;
its compatibility must be established separately. Managed databases must permit
the existing DDL, transactions and advisory-lock operations. The application role
also needs `CREATEDB` when using the create/drop database helpers.

Native Braid trustees communicate with B4 over `B4_URL` (HTTP), and browser
trustees cannot connect directly to PostgreSQL. They do not need PostgreSQL CA
mounts. Their HTTP transport is a separate configuration boundary. The Braid demo
tool connects to PostgreSQL and does use these TLS settings.

PostgreSQL 17 uses the same PostgreSQL SSL negotiation and wire protocol supported
by the pinned `tokio-postgres` 0.7.15 and `postgres-openssl` 0.5.2. No server
extension is required for TLS. See the [B4 tests](./testing/b4.md) for disposable
PostgreSQL tests, including verified TLS and rejection cases.
