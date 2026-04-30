---
id: x509_adding_ca_certificates
title: X.509 — Adding CA Certificates to the Trust Bundle
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# X.509 — Adding CA Certificates to the Trust Bundle

This guide explains how to take a CA certificate issued by an external PKI
(such as a national identity authority), convert it to the correct format, and
add it so that voter certificates signed by that CA are accepted.

**See also:** [X.509 Certificate Voter Authentication](./07-x509-voter-certificate-authentication.md) — full dev and production setup.

---

## 1. Identify the Certificate Format

CA certificates are distributed in two common encodings:

| Format | Description | How to recognise |
|--------|-------------|-----------------|
| **PEM** | Base64-encoded, plain text | Starts with `-----BEGIN CERTIFICATE-----` |
| **DER** | Binary encoding | Starts with hex bytes `30 82` (ASN.1 SEQUENCE) |

Common file extensions (`.cer`, `.crt`, `.der`) do **not** reliably indicate the
encoding — always inspect the file content:

```bash
# If the output starts with -----BEGIN CERTIFICATE-----, it is already PEM.
head -1 <ca-certificate-file>

# Otherwise check the binary header — 30 82 means DER.
xxd <ca-certificate-file> | head -1
```

---

## 2. Convert DER to PEM (if needed)

```bash
openssl x509 -inform DER -in <ca-certificate-file> -out <ca-certificate-file>.pem
```

Verify the result:

```bash
openssl x509 -in <ca-certificate-file>.pem -text -noout | head -20
```

---

## 3. PKCS#7 Bundles

Some authorities distribute certificates as a PKCS#7 file (`.p7b` / `.p7c`
or a PEM file starting with `-----BEGIN PKCS7-----`). Extract all certs with:

```bash
openssl pkcs7 -print_certs -in <bundle-file> -out extracted.pem
```

---

## 4. Add the CA via the Admin Portal

CA certificates are stored in the database per election event and served by
Harvest. This applies to both dev and production. See the [Certificates reference](../../02-election_managers/02-reference/02-election-event/15-election_management_election-event_certificates.md) for details on the admin portal UI.

1. Log in and navigate to the election event
2. Open the **CERTIFICATES** tab
3. Click **Import**, upload the PEM file. If the file is list of concatenated CAs, all of them will be imported.
4. Confirm

Keycloak picks up the new CA within the next refresh cycle (at most
`KC_SPI_TRUSTSTORE_URL_REFRESH_INTERVAL_SECONDS`, default 1 hour in production,
60 seconds in dev). No Keycloak restart is required.

> To force an immediate pickup, reduce the refresh interval temporarily or
> restart Keycloak.

Permissions required: `ca-read` (view) and `ca-write` (add/remove), scoped by
`election_event_id`. `election-event-cas-tab` to allow showing the CAs import tab.


---

## 5. Update the Cloudflare mTLS Truststore (production only)

In production, Cloudflare terminates the TLS connection and validates the client
certificate against its own mTLS truststore **before** forwarding the request to
Keycloak. Cloudflare and Keycloak maintain independent truststores:

| Truststore | Updated by | When |
|---|---|---|
| Keycloak (`UrlTruststoreProvider`) | Admin portal → Postgres → Harvest | Automatic, within refresh cycle |
| Cloudflare mTLS truststore | Operator, manually | Must be done separately |

**When you add or remove a CA via the admin portal, you must also update the
Cloudflare mTLS truststore** in the Cloudflare dashboard (Access → Service Auth
→ mTLS). If this step is skipped, Cloudflare will reject voter certificates
signed by the new CA at the edge, and they will never reach Keycloak.

Note: in dev, nginx uses `optional_no_ca` and performs no CA validation, so
there is no equivalent manual step — only the admin portal import is needed.

---
