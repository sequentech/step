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
add it to the Sequent trust bundle so that voter certificates signed by that CA
are accepted.

**See also:** [X.509 Certificate Voter Authentication](./07-x509-voter-certificate-authentication) — full dev and production setup.

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

If the file is DER-encoded, convert it to PEM before proceeding:

```bash
openssl x509 -inform DER -in <ca-certificate-file> -out <ca-certificate-file>.pem
```

Verify the result:

```bash
openssl x509 -in <ca-certificate-file>.pem -text -noout | head -20
```

You should see human-readable fields such as `Issuer`, `Subject`, and
`Validity`. If you see an error, the source file may be corrupt or in an
unexpected format (e.g. PKCS#7 bundle — see [section 4](#4-pkcs7-bundles)).

---

## 3. Append to the Trust Bundle

The Sequent trust bundle is a single PEM file that concatenates one or more CA
certificates. nginx uses it to verify client certificates at the TLS layer;
Keycloak fetches the same file from MinIO/S3 to re-validate the certificate
chain at the application layer.

To add a new CA, append its PEM to the bundle:

```bash
cat <ca-certificate-file>.pem >> .devcontainer/minio/public-assets/client-ca.pem
```

If the CA has an intermediate certificate that must also be trusted, append it
in the same way — order within the bundle does not matter for OpenSSL chain
verification:

```bash
cat <intermediate-ca-file>.pem >> .devcontainer/minio/public-assets/client-ca.pem
```

Verify the bundle contains the expected entries:

```bash
openssl crl2pkcs7 -nocrl -certfile .devcontainer/minio/public-assets/client-ca.pem \
  | openssl pkcs7 -print_certs -noout
```

This lists each certificate's `Subject` and `Issuer` lines.

---

## 4. PKCS#7 Bundles

Some authorities distribute certificates as a PKCS#7 file (`.p7b` or `.p7c`
extension, or a PEM file starting with `-----BEGIN PKCS7-----`). These are
containers that may hold multiple certificates at once. Extract all certificates
from them with:

```bash
openssl pkcs7 -print_certs -in <bundle-file> -out extracted.pem
```

Then append `extracted.pem` to the trust bundle as in section 3.

---

## 5. Reload Services

After updating the bundle file, restart the relevant services so they pick up
the change:

```bash
# In the dev container — restart nginx and Keycloak
docker compose stop keycloak-nginx keycloak
docker compose up -d --no-deps keycloak-nginx keycloak
```

In production, re-upload the updated PEM to S3. The
`UrlTruststoreProvider` Keycloak SPI will fetch the new bundle on its next
refresh cycle (configurable via `KC_SPI_TRUSTSTORE_URL_REFRESH_INTERVAL_SECONDS`)
without requiring a Keycloak restart.

---

## 6. Verify End-to-End

After reloading, confirm the CA is trusted at both layers:

```bash
# 1. nginx TLS layer — should complete the handshake without SSL error
curl -v --cert <voter-cert.pem> --key <voter-key.pem> \
  https://127.0.0.1:8443/

# 2. Keycloak truststore — check logs for "Fetched N certificate(s)"
docker compose logs keycloak | grep -i "truststore\|certificate"
```

If the voter certificate is signed by the newly added CA and the regex in the
Keycloak X.509 authenticator config correctly extracts the user identifier, the
authentication flow should proceed without error.
