---
id: certificates_tls
title: Certificates and TLS
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

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

**Session: c1.** Run `step 05-001`. Review its implementation with `show 05-001`.

The last command must show `NIST CURVE: P-384` and the three host names.

Send `tls.csr` to a certification authority whose root certificate the common
web browsers trust. Save the certificate you receive, followed by the
intermediate certificates of its chain, in PEM format as
`/root/deploy/tls/tls.crt`.

## Check the certificate

On C1:

<!-- deployment-step: 05-002 -->

**Session: c1.** Run `step 05-002`. Review its implementation with `show 05-002`.

Then check the chain on the administration workstation, against the trust
store of a current browser installation (on Debian and Ubuntu,
`/etc/ssl/certs/ca-certificates.crt`):

<!-- deployment-step: 05-003 -->

**Session: admin.** Run `step 05-003`. Review its implementation with `show 05-003`.

The result must be `leaf.pem: OK`. A certificate that is self-signed, chains
to a private CA, has an RSA key or a curve other than P-384 must not be used:
voters would see a certificate warning, or the profile would not hold.

## Install the certificate

The deployment keeps a certificate Secret named `step-tls-cert` that exists
before it runs, and only generates a self-signed one when there is none.
Create the Secret first, in both namespaces of the TOE.

On C1:

<!-- deployment-step: 05-004 -->

**Session: c1.** Run `step 05-004`. Review its implementation with `show 05-004`.

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

**Session: c1.** Run `step 05-005`. Review its implementation with `show 05-005`.

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

**Session: admin.** Run `step 05-006`. Review its implementation with `show 05-006`.

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
