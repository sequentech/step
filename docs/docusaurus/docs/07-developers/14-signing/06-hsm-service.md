---
id: hsm_service
title: Central HSM Key Service
sidebar_label: HSM key service
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

`packages/hsm-service` is the only component that talks to a key device. Every
other server component is a caller: it reaches keys through
`packages/hsm-client`, which only carries requests. The client holds no key,
links no PKCS#11 or KMS library and does no cryptography of its own. A
workspace test (`hsm-service/tests/boundary.rs`) fails if any other package
depends on a PKCS#11, YubiHSM or KMS crate.

Signing by officials' own tokens is outside this service. For example, EMS
configuration packages are signed on YubiKeys through the local signing
companion, never through the HSM service.

## Backends

`HSM_SERVICE_BACKEND` has no default:

| Value | Device | Notes |
|---|---|---|
| `yubihsm` | YubiHSM 2 through Yubico's `yubihsm_pkcs11` module | The PIN is the authentication key id (4 hex digits) followed by its password. |
| `softhsm` | SoftHSM 2 | A software token. It is refused when `HSM_SERVICE_DEPLOYMENT=production`, and is never reported as hardware. |
| `aws-kms` | AWS KMS | Not available in this build. It needs a KMS account owned by the electoral body, and the service refuses to start with it. |

The configured backend must match the module that answers. A SoftHSM module
is refused under `yubihsm`, and the reverse, so one never stands in for the
other. `GET /v1/health` reports the backend, model and firmware, whether the
device is hardware, and only a SHA-256 prefix of its serial number.

## Settings

| Variable | Meaning |
|---|---|
| `HSM_SERVICE_BACKEND` | `yubihsm`, `softhsm` or `aws-kms` |
| `HSM_SERVICE_DEPLOYMENT` | `test`, `demo` or `production` |
| `HSM_SERVICE_PKCS11_MODULE` | Path of the PKCS#11 module |
| `HSM_SERVICE_TOKEN_LABEL` | Label of the token to use |
| `HSM_SERVICE_PIN_FILE` | File whose first line is the PIN; it must not be readable by group or others |
| `HSM_SERVICE_LISTEN` | Listen address (default `0.0.0.0:8443`) |
| `HSM_SERVICE_TLS_CERT`, `HSM_SERVICE_TLS_KEY` | The service's TLS certificate and key (PEM) |
| `HSM_SERVICE_CLIENT_CA` | CA of the callers' certificates |
| `HSM_SERVICE_POLICY` | Policy file (below) |
| `HSM_SERVICE_STATE_DIR` | Directory for `keys.json` (key versions) and `audit.jsonl` |

## Callers and policy

Callers authenticate with mTLS and are identified by the SHA-256 of their
certificate's public key. Keycloak tokens are not used to authenticate
services. Keycloak's keys, and the vault that holds its secrets, may depend on
this service, so it must start and authorize without Keycloak.

The policy is a JSON file the installation writes. It names:

- the callers;
- the key administrators, who may create, rotate, retire and destroy keys;
- one rule per key id, or per prefix ending in `*`.

Each rule sets the algorithm, operations, purposes, users and a gate:

| Gate | Use | What a signature needs |
|---|---|---|
| `none` | System keys: log entries, checkpoints, HMAC, sealing | An authorized caller and purpose. |
| `completed-execution` | An executor acting on a request whose quorum is complete | The caller must be a listed executor and must name the request and the SHA-256 of the exact message. Both are audited. Checking that the quorum is complete stays with the caller's signing engine, which owns that state. |
| `official-approval` | One official's own approval of a request that is still pending | The official's fresh step-up access token. It is checked here: issuer, audience, signature, `auth_time` within `max_auth_age_seconds`, `acr` in `acr_values`, and optionally `tenant-id`. The token's subject must own the key (`<prefix><subject>`), the request and message digest must be named, and each token (`jti`) authorizes one signature. A completed quorum is not required, because this signature is one of the approvals that make it up. |

The realm's keys for the step-up check are fetched the first time an official
signs, not at start-up.

## Algorithms and exact formats

Callers keep their existing verifiers. Each algorithm signs exactly the bytes
the caller sends:

| Algorithm | Operation | Output |
|---|---|---|
| `ed25519` | Pure Ed25519 (RFC 8032) over the message, never pre-hashed | 64 bytes. These verify with `voting_crypto::signing::VerificationKey`. |
| `ecdsa-p256` | ECDSA P-256 over SHA-256 of the message | Raw `r‖s`, 64 bytes; `formats::ecdsa_raw_to_der` gives the DER form. |
| `hmac-sha256` | HMAC-SHA256 | 32 bytes |
| `ecdh-p256` | ECDH with a peer's uncompressed point | The raw shared secret, for a software KDF. |
| `aes256-gcm` | Seal and open | `iv(12) ‖ ciphertext ‖ tag(16)`, with caller-supplied associated data |

The service adds no prefix or domain tag. A purpose is an authorization
label, not part of the signed bytes, so existing formats stay byte-identical.
New formats define their own domain tags in their own bytes.

`POST /v1/sign-batch` signs each message separately and returns one signature
per message, identical to signing each alone. It exists for throughput:
per-entry signatures are never replaced by a signature over a batch. Only
ungated keys can batch.

## Key versions

A key id names a role, for example the tenant log key. Each version is its own
device object, labelled `<key id>#v<n>` and generated on the device as
sensitive and non-extractable.

- **Rotation** adds a version and retires the active one. Retired versions
  can't sign, but they stay listed with their public keys, so everything they
  signed still verifies.
- **Destruction** removes a retired version's device object and keeps its
  record.
- **Allowed transitions:** only active → retired → destroyed.

## Audit

`audit.jsonl` has one JSON line per operation, whether allowed or refused,
including connections whose certificate the policy doesn't name.

- Each line holds the SHA-256 of the previous one.
- Messages appear only as SHA-256 digests.
- The service won't start on a broken chain.
- An operation whose audit line can't be written fails and returns nothing.
- `hsm-service verify-audit <file>` checks a copy.

The log is independent of the electoral log on purpose. The electoral log's
entries are signed through this service. If each signature were itself an
electoral-log entry, every entry would cause another. Here a signature adds
exactly one audit line.

Not hiding a key's use is different from preventing it. Whoever holds a
caller's certificate, or the device PIN, can use the keys that caller is
allowed. Non-extractability means the keys can't be copied. It doesn't mean
they can't be used.

## Tests

```bash
# Unit tests
cargo test -p hsm-service --lib
# The whole service on SoftHSM over mTLS (needs softhsm2 installed)
HSM_SERVICE_TEST_PKCS11_MODULE=/usr/lib/softhsm/libsofthsm2.so cargo test -p hsm-service
```

A pass on SoftHSM says nothing about a YubiHSM 2. The YubiHSM acceptance run
uses the device itself.
