---
id: postgresql-tls
title: Native board PostgreSQL TLS
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The VoteSecure board (`b4v6`) and Windmill's direct board archive pool use SQLx
with Rustls. PostgreSQL URLs default to `sslmode=verify-full`: the connection
requires TLS, validates the certificate chain, and verifies the database host.
A server that refuses TLS fails the connection; there is no plaintext retry.

Set `DATABASE_URL` for the board server and `WBRAID_BOARD_DATABASE_URL` for
Windmill to the same database. The supplied Compose configurations accept
`WBRAID_BOARD_DATABASE_URL` as the override for both services. Keep passwords
in `WBRAID_B4_PG_PASSWORD`, rather than committing credentials in URLs.

For a managed PostgreSQL endpoint with a private CA, use its certified hostname
and provide the CA file through a read-only mount in both containers:

```text
postgres://board_user@db.example.com:5432/board?sslmode=verify-full&sslrootcert=/certs/provider-ca.pem
```

URL-encode reserved characters in usernames, database names and certificate
paths. SQLx also accepts `ssl-mode`, `ssl-root-cert` and `ssl-ca`. Publicly trusted
certificates use SQLx/Rustls's bundled WebPKI roots when no custom CA is supplied. A missing
or invalid CA file, an untrusted certificate or a hostname mismatch fails the
connection. Installing a private CA in the image's operating-system trust store does not
replace the explicit `sslrootcert` setting for this Rustls connector. Helm deployments
must pass these URLs through the service environment and mount the CA read-only
at the same configured path in each service that opens a database connection.

Only `verify-full` and `disable` are accepted. Empty, repeated, unsupported and
opportunistic modes fail configuration validation. The application's URL policy
takes precedence over the driver's generic `PGSSLMODE` environment default.
Other PostgreSQL credentials and certificate URL options retain SQLx semantics.

`sslmode=disable` is an explicit exception for a trusted local development
PostgreSQL server or the loopback hop to an authenticated provider proxy. The
bundled Compose database uses this explicit mode. A Cloud SQL Auth Proxy secures
its upstream GCP connection; it is not a general proxy for other providers.
A remote managed database should use `verify-full` and its provider's trust roots.
Do not use `disable` for a remote database merely to bypass a certificate error.

Trustees and the ordinary protocol-board client call the board's HTTP API and do
not open its PostgreSQL database. Application-owned pools supplied through
`BoardDb::from_postgres_pool` retain their owner's connection policy; application
code should build them using `postgres_connect_options`.
