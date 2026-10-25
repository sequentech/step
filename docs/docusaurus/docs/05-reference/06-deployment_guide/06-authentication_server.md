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
| Claims | the user name and e-mail address of each person, as their account in the TOE will carry them |
| Authentication | two factors for administrators and election board members |

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

```bash
. /root/deploy/hosts.env
cd /root/deploy && umask 077
read -r -s -p 'Client secret: ' IDP_SECRET && echo && export IDP_SECRET
jq -n --arg alias "$IDP_ALIAS" --arg issuer '<issuer>' \
  --arg auth '<authorization-endpoint>' --arg token '<token-endpoint>' \
  --arg jwks '<jwks-url>' --arg logout '<end-session-endpoint>' \
  --arg id '<client-id>' '{
  alias: $alias, displayName: $alias, providerId: "oidc", enabled: true,
  trustEmail: false, storeToken: false, linkOnly: false,
  firstBrokerLoginFlowAlias: "certificate-first-login-flow",
  config: {issuer: $issuer, authorizationUrl: $auth, tokenUrl: $token,
    jwksUrl: $jwks, logoutUrl: $logout, clientId: $id,
    clientSecret: $ENV.IDP_SECRET, clientAuthMethod: "client_secret_post",
    validateSignature: "true", useJwksUrl: "true",
    pkceEnabled: "true", pkceMethod: "S256"}}' > idp.json
unset IDP_SECRET
jq 'del(.config.clientSecret)' idp.json
```

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

```bash
openssl x509 -in /root/deploy/auth-server-ca.pem -noout -subject -fingerprint -sha256
k3s kubectl -n step-infra create configmap auth-server-ca \
  --from-file=ca.pem=/root/deploy/auth-server-ca.pem
```

The fingerprint must be the one you received. [Configuring the
Deployment](07-hardening.md) makes Keycloak use this certificate and limits
its connections to the TLS profile of [Certificates and TLS](05-certificates_tls.md).

## Check the authentication server's TLS

On C1:

```bash
. /root/deploy/hosts.env
ca=/root/deploy/auth-server-ca.pem
tls() { echo | openssl s_client -connect "$AUTH_HOST:443" -servername "$AUTH_HOST" \
  -CAfile "$ca" -verify_hostname "$AUTH_HOST" -verify_return_error "$@" 2>&1; }
tls -tls1_3 -ciphersuites TLS_AES_256_GCM_SHA384 -groups secp384r1 \
  | grep -E '^New,|Temp Key|public key is|Verify return'
tls -tls1_2 >/dev/null && echo "TLS 1.2 ACCEPTED" || echo "TLS 1.2 refused"
tls -tls1_3 -ciphersuites TLS_AES_128_GCM_SHA256 >/dev/null \
  && echo "AES-128 ACCEPTED" || echo "AES-128 refused"
tls -tls1_3 -groups X25519 >/dev/null && echo "X25519 ACCEPTED" || echo "X25519 refused"
```

The first command must show `New, TLSv1.3, Cipher is TLS_AES_256_GCM_SHA384`,
a temporary key on `secp384r1`, `Server public key is 384 bit` and
`Verify return code: 0 (ok)`; every other line must say `refused`. On C2,
check that the server is reachable, since Keycloak can run on either node:

```bash
timeout 5 bash -c "</dev/tcp/<authentication-server-host>/443" && echo reachable
```
