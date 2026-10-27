---
id: certificates_tls
title: Certificates and TLS
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Read this chapter for the requirements and expected results behind the
[configuration-driven installation](01-about.md#install-from-one-configuration-file).
Follow the explicit commands and human checkpoints below. The installer performs
the server-side operations; you do not run internal numbered operations yourself.

Every connection to the TOE's web hosts uses one TLS profile:

| Property | Required value |
| --- | --- |
| Protocol | TLS 1.3 only |
| Cipher suite | TLS_AES_256_GCM_SHA384 only |
| Key exchange group | secp384r1 only |
| Server certificate | ECDSA key on the P-384 curve |
| Plain HTTP | none: port 80 closed |

The same profile applies to the external authentication server (see
[Connecting the Authentication Server](06-authentication_server.md)).

## Create the key and the certificate request

:::warning
The private key never leaves C1. Create it there, in the root-only deployment
directory.
:::

On C1:

<!-- deployment-step: 05-001 -->

**Expected result.** The certificate request summary shows `NIST CURVE: P-384` and DNS entries for all three configured web host names. The private key is saved locally; it is not printed.

**If the result differs.** If a host name is wrong, correct the controlled site configuration before requesting a certificate. If key generation or request creation fails, inspect the root-only TLS directory and installed OpenSSL prerequisites. Do not blindly rerun this step after submitting a CSR: it replaces the private key, invalidating the relationship to that request.


The last command must show `NIST CURVE: P-384` and the three host names.

Send `tls.csr` to a certification authority whose root certificate the common
web browsers trust. Save the certificate you receive, followed by the
intermediate certificates of its chain, in PEM format as
`/root/deploy/tls/tls.crt`.

## Check the certificate

On C1:

<!-- deployment-step: 05-002 -->

**Expected result.** The certificate summary lists the intended subject, issuer, all three DNS names and a future expiry date. It shows `NIST CURVE: P-384`, then `certificate matches the key`.

**If the result differs.** If the certificate cannot be parsed, check that the CA response and chain were saved in PEM format. For missing names, expiry, a wrong curve or a key mismatch, obtain the correct certificate for the existing CSR. Never replace or export the private key to make an unrelated certificate fit.


Then check the chain on the administration workstation, against the trust
store of a current browser installation (on Debian and Ubuntu,
`/etc/ssl/certs/ca-certificates.crt`):

<!-- deployment-step: 05-003 -->

**Expected result.** The final verification line is `leaf.pem: OK`. The certificate chain reaches the administration workstation’s current public browser-compatible CA trust store.

**If the result differs.** For `unable to get local issuer certificate`, check the supplied intermediate chain. For expiry or an untrusted/private CA, obtain a suitable certificate from the CA. Do not add a private CA to this check’s trust store merely to force `OK`; voters’ ordinary browsers must trust the chain.


The result must be `leaf.pem: OK`. A certificate that is self-signed, chains
to a private CA, has an RSA key or a curve other than P-384 must not be used:
voters would see a certificate warning, or the profile would not hold.

## Install the certificate

The deployment keeps a certificate Secret named `step-tls-cert` that exists
before it runs, and only generates a self-signed one when there is none.
Create the Secret first, in both namespaces of the TOE.

On C1:

<!-- deployment-step: 05-004 -->

**Expected result.** Namespace apply messages are followed by creation of `step-tls-cert` in each namespace. The final table lists that TLS Secret in both `step-apps` and `step-infra` (type `kubernetes.io/tls`).

**If the result differs.** If a Secret already exists, stop and establish whether this is a renewal or a partial earlier installation; do not delete it blindly. For a certificate/key error, return to certificate verification. If only one namespace succeeded, inspect both namespaces and reconcile the partial state before proceeding.


The last command must list `step-tls-cert` in `step-apps` and in
`step-infra`.

## Set up the TLS endpoint

:::warning
Every connection from the network must end at the TLS endpoint, which alone
enforces the TLS profile above. The cluster's ingress behind it listens only on
C1's loopback interface and must never be reachable from the network.
:::

The TLS endpoint is the `nginx` service of C1's operating system, installed
with the servers from the installation kit. It is the only service that
listens on port 443 of C1's network addresses. It accepts only connections
that meet the TLS profile and name one of the three host names (SNI), and
forwards each one over C1's loopback interface, encrypted and with the
client's address, to the cluster's ingress, which routes it to the TOE. No
service listens on port 80. The TLS endpoint reads its certificate and key
from `/var/lib/sequent-site/tls/`.

On C1, install the certificate and the key, and restart the TLS endpoint:

<!-- deployment-step: 05-005 -->

**Expected result.** The last two output lines are `active` and `nginx`. The second line identifies the sole process listening on TCP 443; no other process name may appear.

**If the result differs.** If nginx fails to restart, inspect its service status and journal for certificate-path or configuration errors; retain private key confidentiality. If another process occupies port 443, stop and reconcile the installed kit’s endpoint arrangement. Do not bypass nginx or expose the ingress directly.


The last two commands must print `active` and `nginx` alone. Repeat this
section each time the certificate is renewed, together with
[Install the certificate](#install-the-certificate) (delete the two
`step-tls-cert` Secrets first and deploy again afterwards, as in
[Deploy](08-installing_toe.md#deploy)).

## Check the TLS profile

Run these checks from the administration workstation once the TLS endpoint
has its certificate, and again in
[Verifying the Installation](10-verification.md):

<!-- deployment-step: 05-006 -->

**Expected result.** For each of the three host names, the successful handshake reports TLS 1.3, `TLS_AES_256_GCM_SHA384`, a `secp384r1` temporary key, a 384-bit server public key and `Verify return code: 0 (ok)`. All prohibited protocol/cipher/group and wrong-or-missing-name attempts say `refused`. The final C1 port-80 line says `closed`.

**If the result differs.** Any `ACCEPTED`, `open`, missing positive-handshake field or certificate verification error fails this check. Verify DNS/SNI host names, certificate chain and the controlled TLS endpoint configuration. A disconnected endpoint can refuse everything, so negative results never replace the successful allowed handshake.


For each host, the first command must print four lines:
`Peer Temp Key: ECDH, secp384r1, 384 bits` (older OpenSSL versions say
`Server Temp Key`), `New, TLSv1.3, Cipher is TLS_AES_256_GCM_SHA384`,
`Server public key is 384 bit` and `Verify return code: 0 (ok)`. Every other
line must say `refused`, and port 80 must be `closed`. If any line says
`ACCEPTED` or `open`, or a line is missing, the TLS profile does not hold:
stop and report it to [security@sequentech.io](mailto:security@sequentech.io).

[Network filtering](04-hosts_network_time.md#network-filtering) and
[Verifying the Installation](10-verification.md#tls-endpoint-and-ingress-ports)
check from a client network that the ingress's ports are closed.
