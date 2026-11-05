---
id: election_management_election_event_signatures_certificates
title: Signing certificates
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

**Signatures** > **Certificates** decides which staff certificates can sign: the trusted
issuers, the checks every signature passes, and the certificates registered to people.
Reading it needs `signing-certificates-read`; each kind of change has its own permission.

These are **staff** certificates. They are separate from the certificate authorities
voters sign in with, which live in the election event's
[Certificates](../15-election_management_election-event_certificates.md) tab.

## Trusted issuers

Staff certificates must chain to one of the trusted issuers. Until one is imported, nobody
can sign.

- **Import issuer certificates** (`signing-issuers-write`): choose a PEM or CER file. A PEM
  file can hold several certificates; certificates already trusted are skipped, and
  refused ones are listed.
- Each issuer shows its name, **Type** (Root or Intermediate), **Issued by**, **Valid
  until** and its SHA-256 fingerprint.
- **Remove** (`signing-issuers-write`): certificates the issuer issued can no longer sign.
  Signatures already given stay recorded.

Import every certificate authority that issues signers' certificates, the intermediate
ones included: revocation lists are only checked for issuers that are imported.

## Checks

Changing the checks needs `signing-checks-write`.

| Setting | Options |
|---|---|
| **Check revocation lists** | On: each signer's certificate is checked against its issuer's revocation list. The lists are downloaded from the addresses in the certificates every hour, and each list shows when it was last downloaded or that it couldn't be. |
| **When a list can't be downloaded** | **Don't accept signatures**, or **Accept and mark the signature as unchecked**. An unchecked signature is recorded as such. |
| **Registering a certificate to a person** | **When its holder first signs with it**, or **Only when someone who can register certificates registers it**. |
| **A certificate signs for one Post only** | On: once a certificate has signed for a Post, it can't sign for another Post of the event. Event-level actions don't count. |

The defaults are: check revocation lists, don't accept signatures without a list, register
on first use, one Post per certificate.

### What every signature is checked for

The server repeats every check when someone signs; the signer sees the same list in the
signing dialog before signing.

| Check | Passes when |
|---|---|
| Issued by a trusted issuer | The certificate chains to a trusted issuer, with current algorithms (RSA keys of at least 2048 bits, or EC P-256). |
| Valid today | Today is within the certificate's validity dates. |
| Made for signing | Its key usage allows digital signatures or non-repudiation, and its extended key usage, if it has one, allows signing. |
| Not revoked | It is not on its issuer's current revocation list (see the settings above). |
| Registered to you | It is registered to the signer, or will be on first use. |
| Not registered to anyone else | Neither the certificate, its key nor its holder's name is registered to another account in the tenant. |
| Not used for this request yet | Neither the signer, the certificate, its key nor its holder has already signed this request. |
| Registered for this Post | With **one Post only**, it hasn't signed for another Post. |
| The signature covers this request | The signature is over exactly what the request signs. |

A request needs that many **different people**: the same person can't fill two places,
whether they sign twice, sign in again, or use their certificate from someone else's
account.

## Registered certificates

The list shows every certificate registered to a person in this election event: the
**Person** (name, username and role), the **Post** (or **All** for event-level signers),
the **Certificate** (its name and a short fingerprint), the **Issuer**, **Valid until**, how
it was **Registered** (**On first signature**, or by whom) and its **Status**: Active,
**Expires soon** (within 30 days), Expired or Revoked with the date. Search by person,
certificate or Post, and filter by status.

### Registering a certificate

**Register a certificate** (`signing-certificates-register`): find the person by username,
choose the Post (or none, for someone who signs event-level actions), and paste the
certificate as PEM. The certificate is refused when its issuer isn't trusted, when it
isn't valid today, or when it isn't made for signing.

When the certificate, its key or its holder is already registered to another account, the
dialog says so. If both accounts belong to the same person (for example a board member who
is also a trustee), **Link as a second account of the same person** registers it to the
second account too. The two accounts still count as one signer in any request.

With **Only when someone who can register certificates registers it**, signers must have
their certificate registered here before they can sign.

### Revoking a certificate

**Revoke** (`signing-certificates-revoke`) asks for a reason. Revoking works on the
certificate's **key**, across the tenant:

- every registration of the same key, in every election event of the tenant, is revoked,
  and each event logs it;
- waiting requests the key already signed are cancelled (reason: the certificate was
  revoked), because their signatures no longer count;
- requests that already completed keep their signatures;
- the certificate and its key can never sign or be registered again. The person needs a
  new certificate with a new key.

## What is logged

`SigningIssuerChanged`, `SigningChecksChanged` (with the old and new values),
`SigningCertificateRegistered` and `SigningCertificateRevoked` (with the reason), each as a
USER and a SYSTEM entry in the [Logs](../11-election_management_election-event_logs.md).
