---
id: b3_postgresql_tls
title: B3 PostgreSQL TLS
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# B3 PostgreSQL TLS

The B3 server, Windmill's direct B3 client, and the native Braid demo tools
share the same PostgreSQL transport policy. Trustees connect to B3 over gRPC;
these PostgreSQL settings apply to processes that open database sessions.

| Variable | Default | Meaning |
| --- | --- | --- |
| `B3_PG_SSL_MODE` | `verify-full` | Require TLS with a trusted certificate chain and matching hostname. `require` is an alias with the same verification guarantees. `disable` explicitly selects plaintext. |
| `B3_PG_CA_PATH` | unset | Optional path to a PEM CA bundle inside the process/container. OpenSSL system roots are used when unset. |

There is no fallback to plaintext. Unsupported modes, unreadable/invalid CA
bundles, untrusted certificates, hostname mismatches, and endpoints refusing TLS
fail the connection. A CA path cannot be combined with `disable`.

For a managed PostgreSQL endpoint, configure its certificate's DNS hostname in
`B3_PG_HOST`, set `B3_PG_SSL_MODE=verify-full`, and mount the provider's CA bundle
read-only into **both B3 and Windmill** when it is not trusted by system roots:

```dotenv
B3_PG_HOST=postgres.example.org
B3_PG_PORT=5432
B3_PG_SSL_MODE=verify-full
B3_PG_CA_PATH=/etc/postgresql/provider-ca.pem
```

The supplied Compose files pass these variables to the database consumers.
A custom CA file must be mounted at the specified path in each consumer; a host
filesystem path alone does not make the file available in a container. Keep the
existing database credentials outside this example. PostgreSQL 17 uses the
standard PostgreSQL TLS negotiation; certificate names and the provider's trust
chain must still match the configured endpoint.

The development, remote example, and airgap `.env` files explicitly use
`B3_PG_SSL_MODE=disable` for their bundled plaintext PostgreSQL containers.
`disable` also supports an authenticated local Cloud SQL Auth Proxy with
`B3_PG_HOST=127.0.0.1`; its upstream encryption is handled by that proxy. Do not
point this plaintext mode at an external database. PgBouncer can be used when
needed, but native TLS can connect directly to a managed endpoint.

Library callers use `PgsqlConnectionParams::with_tls` to configure the policy.
Connection fields are passed as typed values rather than interpolated into a
libpq connection string. Pool size and transaction/session ownership remain
unchanged, including PostgreSQL advisory-lock lifetimes.

## Local TLS regression suite

The ignored integration tests create isolated PostgreSQL clusters on loopback
with short-lived synthetic certificates. Run them explicitly on a non-root
host with PostgreSQL and OpenSSL installed:

```sh
cd packages
PG_BIN=/path/to/postgresql/bin OPENSSL_BIN=/path/to/openssl \
  cargo test -p b3 --features server,sqlcopy --test postgres_tls -- --ignored
```

They exercise verified direct, pooled and server connections, database
provisioning, rejection of invalid trust/hostnames and plaintext endpoints,
and advisory-lock ownership.
