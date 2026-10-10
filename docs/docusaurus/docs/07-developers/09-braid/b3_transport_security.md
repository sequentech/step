---
id: b3_transport_security
title: Bulletin Board Transport Security
sidebar_label: Bulletin Board Transport Security
---

# Bulletin Board Transport Security

Braid trustees read and post protocol messages through the b3 bulletin board
gRPC service. `B3_TRANSPORT_SECURITY` on b3 and `B3_CLIENT_TRANSPORT_SECURITY`
on each trustee select how that connection is protected:

| Value | Behaviour |
|-------|-----------|
| `plaintext` | Default. Unencrypted HTTP/2. b3 serves any client that reaches its port. |
| `tls` | b3 presents a server certificate that clients verify against a CA. Any client that completes the TLS handshake is served. |
| `mutual_tls` | Like `tls`, and b3 also requires a client certificate issued by its configured CA. Connections without one are refused during the TLS handshake. |

Use `mutual_tls` whenever hosts other than the trustees can reach the b3 port.
When b3 runs with `plaintext` on a non-loopback address it logs a warning at
startup.

Windmill reads b3 through its PostgreSQL database, not through the gRPC
service, so it needs no change.

## b3 server

| Variable | Description | Required with |
|----------|-------------|---------------|
| `B3_TRANSPORT_SECURITY` | `plaintext`, `tls` or `mutual_tls`. Defaults to `plaintext`. | - |
| `B3_TLS_CERT_PATH` | PEM certificate chain of b3. Its subject alternative names must include the host of the trustees' `B3_URL`. | `tls`, `mutual_tls` |
| `B3_TLS_KEY_PATH` | PEM private key of that certificate. | `tls`, `mutual_tls` |
| `B3_TLS_CA_PATH` | PEM certificate of the CA that issues the trustee client certificates. | `mutual_tls` |

## Trustees

The braid trustee and the braid `verify` tool read these variables:

| Variable | Description | Required with |
|----------|-------------|---------------|
| `B3_CLIENT_TRANSPORT_SECURITY` | Must match `B3_TRANSPORT_SECURITY` on b3. Defaults to `plaintext`. | - |
| `B3_CLIENT_TLS_CA_PATH` | PEM certificate of the CA that issued the b3 server certificate. | `tls`, `mutual_tls` |
| `B3_CLIENT_TLS_CERT_PATH` | PEM client certificate of this trustee, issued by the CA in b3's `B3_TLS_CA_PATH`. | `mutual_tls` |
| `B3_CLIENT_TLS_KEY_PATH` | PEM private key of that certificate. | `mutual_tls` |

With `tls` or `mutual_tls`, `B3_URL` must use the `https://` scheme, for example
`https://b3:50051`. The trustee refuses an `http://` URL in these modes.

## Enabling mutual TLS

1. Issue a server certificate for b3 whose subject alternative names include
   the host name the trustees use in `B3_URL` (for example `b3`), and one client
   certificate per trustee from a CA reserved for b3 clients. Client
   certificates that carry an extended key usage must include `clientAuth`.
2. Make the certificate and key files readable inside the b3 and trustee
   containers. The Docker Compose files in this repository already pass the
   variables listed above to b3 and the trustees; add read-only volume mounts
   for the files, for example in a `docker-compose.override.yml`.
3. On b3 set `B3_TRANSPORT_SECURITY=mutual_tls`, `B3_TLS_CERT_PATH`,
   `B3_TLS_KEY_PATH` and `B3_TLS_CA_PATH`.
4. On every trustee set `B3_CLIENT_TRANSPORT_SECURITY=mutual_tls`,
   `B3_CLIENT_TLS_CA_PATH`, `B3_CLIENT_TLS_CERT_PATH`, `B3_CLIENT_TLS_KEY_PATH`,
   and change `B3_URL` to `https://`.
5. Restart b3 and the trustees. A trustee whose settings do not match b3 logs
   `Error listing board names` and retries until they do.

Existing boards and trustee configurations are not affected by the change of
transport.

The development and air-gap preparation Docker Compose files publish the b3
port on `127.0.0.1` only. The trustees reach b3 over the Compose network.
