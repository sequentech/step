---
id: authentication_server
title: Connecting the Authentication Server
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Read this chapter for the requirements and expected results behind the
[configuration-driven installation](01-about.md#install-from-one-configuration-file).
Operation references identify implementation details and troubleshooting checks;
they are not additional shell commands to type after a successful installation.

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

Before running the installer, prepare the protected JSON file named by
`identity_provider.definition` in `site.json`. Use a local text editor on the
administration workstation; do not put the client secret in a command argument,
terminal output or public configuration. Keep the file owned by root and readable
only by root. Fill the registration values supplied by the identity-provider
operator; the following values are placeholders:

```json
{
  "alias": "external-login",
  "displayName": "external-login",
  "providerId": "oidc",
  "enabled": true,
  "trustEmail": false,
  "storeToken": false,
  "linkOnly": false,
  "firstBrokerLoginFlowAlias": "certificate-first-login-flow",
  "config": {
    "issuer": "https://login.example.org/realms/example",
    "authorizationUrl": "https://login.example.org/realms/example/protocol/openid-connect/auth",
    "tokenUrl": "https://login.example.org/realms/example/protocol/openid-connect/token",
    "jwksUrl": "https://login.example.org/realms/example/protocol/openid-connect/certs",
    "logoutUrl": "https://login.example.org/realms/example/protocol/openid-connect/logout",
    "clientId": "REPLACE_WITH_REGISTERED_CLIENT_ID",
    "clientSecret": "REPLACE_WITH_CLIENT_SECRET_IN_THIS_PRIVATE_FILE_ONLY",
    "clientAuthMethod": "client_secret_post",
    "validateSignature": "true",
    "useJwksUrl": "true",
    "pkceEnabled": "true",
    "pkceMethod": "S256"
  }
}
```

Both alias fields must match `idp_alias` in `site.json`. Use the actual endpoint
URLs from the registration; do not derive or guess them from these examples. The
installer copies the definition through the protected administration connection.
The client secret remains in the private definition, not in `site.json`.

<!-- deployment-step: 06-001 -->

**Operation reference:** `06-configure-external-login` on `c1`.

**Expected result.** Configuration validation accepts the private definition, including its issuer, HTTPS endpoint URLs, client ID, signature-validation settings and `certificate-first-login-flow`. The client secret is not printed.

**If the result differs.** If the public values differ from the independently supplied registration, stop before applying the provider definition. Correct the private file in the editor and rerun the check. Never paste its unredacted contents into a report or put the client secret in the public site file.


Check the public registration values against those you received. `validateSignature` and
`useJwksUrl` make the TOE accept only ID tokens signed with the server's
published keys, and `issuer` makes it accept only tokens of that issuer. The
first-login flow `certificate-first-login-flow` links a sign-in to an existing
account and never creates one.

## Install the trust anchor

:::warning
Keycloak trusts the authentication server's TLS certificate only through this
CA certificate. Check its fingerprint before installing it.
:::

Save the CA certificate as `/root/deploy/auth-server-ca.pem` on C1. **Before installing
it**, run this short read-only check in the C1 session:

```bash
openssl x509 -in /root/deploy/auth-server-ca.pem -noout -subject -fingerprint -sha256
```

Expected: the subject and SHA-256 fingerprint supplied independently by the
authentication-server operator. Compare every fingerprint byte. If they differ,
stop and obtain the correct CA certificate through that operator's verified channel.
The installation command below repeats the fingerprint and then immediately creates
the ConfigMap; it does not pause to ask you to approve the fingerprint.

Only after that independent comparison passes, install the trust anchor:

<!-- deployment-step: 06-002 -->

**Operation reference:** `06-install-idp-ca` on `c1`.

**Expected result.** The certificate subject and SHA-256 fingerprint match the independently received CA certificate details, followed by `configmap/auth-server-ca created`. The fingerprint must already have passed the separate precheck above before this command is run.

**If the result differs.** If the precheck differs, do not run this step. If creation says `AlreadyExists`, inspect whether this is a partial earlier setup and verify the existing trusted certificate before replacing anything. A missing namespace means the certificate/namespace installation stage is incomplete; return to it first.


The fingerprint must be the one you received. [Configuring the
Deployment](07-hardening.md) makes Keycloak use this certificate and limits
its connections to the TLS profile of [Certificates and TLS](05-certificates_tls.md).

## Check the authentication server's TLS

On C1:

<!-- deployment-step: 06-003 -->

**Operation reference:** `06-check-idp-tls` on `c1`.

**Expected result.** The allowed handshake reports TLS 1.3, `TLS_AES_256_GCM_SHA384`, a `secp384r1` temporary key, a 384-bit server public key and `Verify return code: 0 (ok)`. The TLS 1.2, AES-128 and X25519 attempts each say `refused`.

**If the result differs.** If the allowed handshake fails, check the independently verified CA file, DNS, endpoint host name and firewall path. If a prohibited attempt is accepted, have the external authentication-server operator correct its TLS profile before connecting the TOE. Do not disable certificate or host-name verification.


The first command must show `New, TLSv1.3, Cipher is TLS_AES_256_GCM_SHA384`,
a temporary key on `secp384r1`, `Server public key is 384 bit` and
`Verify return code: 0 (ok)`; every other line must say `refused`. On C2,
check that the server is reachable, since Keycloak can run on either node:

<!-- deployment-step: 06-004 -->

**Operation reference:** `06-check-c2-idp-access` on `c2`.

**Expected result.** The single success message is `reachable`. This is a TCP reachability check from C2, not an authentication or certificate validation result.

**If the result differs.** If no `reachable` message appears or the command times out, check C2’s DNS and outbound TCP-443 route to the configured authentication host. Resolve the network problem before allowing Keycloak to run there; do not change the host to an unrelated reachable server.
