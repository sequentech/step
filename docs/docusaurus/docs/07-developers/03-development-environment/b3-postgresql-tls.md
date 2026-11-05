<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# B3 PostgreSQL TLS

B3's server pool, direct clients, database creation/deletion helpers, and
Windmill's B3 archive connection use the same PostgreSQL transport policy.
`B3_PG_SSL_MODE` defaults to `verify-full`: TLS is required, the certificate
chain must be trusted, and the certificate must match `B3_PG_HOST`.
A refused TLS handshake is an error; connections never fall back to plaintext.
`require` is an alias for the same verified policy. Modes such as `prefer`,
`allow` and `verify-ca` are rejected.

Set `B3_PG_CA_PATH` to a readable PEM CA bundle for a private or managed
provider's CA. Without it, the runtime's OpenSSL system roots are used.
Use the provider's DNS endpoint as `B3_PG_HOST`; an IP address needs a matching
IP subject alternative name. A missing, unreadable, or invalid CA file fails
before connecting. Credentials and database names are passed as typed values,
so spaces, quotes and connection-option text cannot alter the TLS policy.

The same settings must reach both the B3 server and Windmill. Shipped Compose
files pass them through. Mount the CA read-only at the configured container
path in each service, for example in a Compose override:

```yaml
services:
  b3:
    environment:
      B3_PG_SSL_MODE: verify-full
      B3_PG_CA_PATH: /etc/b3/ca.pem
    volumes:
      - ./certificates/postgresql-ca.pem:/etc/b3/ca.pem:ro
  windmill:
    environment:
      B3_PG_SSL_MODE: verify-full
      B3_PG_CA_PATH: /etc/b3/ca.pem
    volumes:
      - ./certificates/postgresql-ca.pem:/etc/b3/ca.pem:ro
```

The bundled development/standalone database uses an explicit
`B3_PG_SSL_MODE=disable`. This mode is also available for a trusted local
Cloud SQL Auth Proxy or equivalent authenticated TLS bridge. Select it only
for that isolated local connection and clear `B3_PG_CA_PATH`; the combination
of `disable` with a CA path is rejected. Unix sockets also require explicit
`disable`. No deployed configuration is changed by this code.

Native trustee processes connect to B3 over gRPC and do not open PostgreSQL
sessions. The Braid demo tool's direct PostgreSQL operations read
`B3_PG_SSL_MODE` and `B3_PG_CA_PATH` like Windmill. For its local demo database,
set `B3_PG_SSL_MODE=disable` explicitly.

## Local transport tests

The integration suite creates disposable loopback PostgreSQL servers and
synthetic CA certificates. It needs `initdb`, `pg_ctl`, and OpenSSL on PATH;
`PG_BIN` and `OPENSSL_BIN` may point to their installation paths. Run as a
non-root user:

```sh
cd packages
CARGO_TARGET_DIR="$PWD/rust-local-target" cargo test -p b3 --features 'server sqlcopy' --test postgres_tls
```

The tests cover trusted TLS, wrong/untrusted CAs, hostname mismatch, a server
that refuses TLS, explicit local plaintext, invalid/missing configuration,
server and direct/pooled clients, create/drop helpers, and advisory-lock session
and transaction lifetimes. Managed PostgreSQL additionally needs network
access, the provider's CA bundle, and a hostname covered by its certificate.
Image builds and provider endpoints need separate deployment validation.
