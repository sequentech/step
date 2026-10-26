---
id: authentication_server
title: Connecting the Authentication Server
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Administrators, election board members and voters sign in only through the
external authentication server. The TOE's identity service (keycloak) sends
every sign-in there, accepts the result only from that server, and links it
only to an account that already exists in the TOE. This page prepares that
connection; [Configuring the Deployment](07-hardening.md) applies it.

## Register the TOE at the authentication server

Ask the operator of the authentication server to register the TOE as a client
with these properties:

| Property | Value |
| --- | --- |
| Protocol | OpenID Connect, authorization code flow |
| Client type | confidential, with a client secret sent in the request body |
| PKCE | required, method S256 |
| Redirect URI of the super-admin tenant | `https://PORTAL_HOST/realms/R/broker/IDP_ALIAS/endpoint`, with the values of PORTAL_HOST, R and IDP_ALIAS |
| Redirect URI of the customer's tenant | the same, with the name of the customer's realm (CR) in place of R; registered when the tenant is created (see [Create the customer's tenant](08-installing_toe.md#create-the-customers-tenant)) |
| Redirect URI of each election event | the same, with the name of the event's realm in place of R; registered when the event is created (see [Self-Test and Handover](11-selftest_handover.md)) |
| Claims | the user name and e-mail address of each person, as their account in the TOE will carry them; the actual authentication time in the signed ID token (`auth_time`) |
| Authentication | two factors for administrators and election board members |
| Fresh authentication | honour `prompt=login` with a fresh login, including both factors for a board member, even when an upstream session exists; honour `max_age` and return the actual authentication time, not the token issue time |

:::warning
Register exact redirect URIs only, never a wildcard. The authentication
server must refuse every other redirect URI for this client.
:::

From the operator of the authentication server, obtain, through a channel
that protects their integrity: the issuer, the authorization endpoint, the
token endpoint, the URL of its signing keys (JWKS), the end-session endpoint,
the client ID and the client secret of the TOE's registration, and the CA
certificate its server certificate chains to, with the SHA-256 fingerprint of
that CA certificate.

## Write the provider definition

:::warning
The provider definition holds the client secret. Keep it only in the
root-only deployment directory.
:::

On C1, write the definition with your values. The command asks for the client
secret without showing it:

<!-- deployment-step: 06-001 -->

**Session: c1.** Run `step 06-001`. Review its implementation with `show 06-001`.

Check the output against the values you received. `validateSignature` and
`useJwksUrl` make the TOE accept only ID tokens signed with the server's
published keys, and `issuer` makes it accept only tokens of that issuer. The
first-login flow `certificate-first-login-flow` links a sign-in to an existing
account and never creates one.

## Install the trust anchor

:::warning
Keycloak trusts the authentication server's TLS certificate only through this
CA certificate. Check its fingerprint before installing it.
:::

Save the CA certificate as `/root/deploy/auth-server-ca.pem` on C1, then:

<!-- deployment-step: 06-002 -->

**Session: c1.** Run `step 06-002`. Review its implementation with `show 06-002`.

The fingerprint must be the one you received. [Configuring the
Deployment](07-hardening.md) makes Keycloak use this certificate and limits
its connections to the TLS profile of [Certificates and TLS](05-certificates_tls.md).

## Check the authentication server's TLS

On C1:

<!-- deployment-step: 06-003 -->

**Session: c1.** Run `step 06-003`. Review its implementation with `show 06-003`.

The first command must show `New, TLSv1.3, Cipher is TLS_AES_256_GCM_SHA384`,
a temporary key on `secp384r1`, `Server public key is 384 bit` and
`Verify return code: 0 (ok)`; every other line must say `refused`. On C2,
check that the server is reachable, since Keycloak can run on either node:

<!-- deployment-step: 06-004 -->

**Session: c2.** Run `step 06-004`. Review its implementation with `show 06-004`.
