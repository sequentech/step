---
id: authentication_server
title: Connecting the Authentication Server
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This chapter prepares the identity-provider registration and protected input
files. The [configuration-driven installer](01-about.md#install-from-one-configuration-file)
checks them, then configures the connection automatically. The provider's operator
performs the external registration; the installer cannot authorize it for them.

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
| Redirect URI of the super-admin tenant | `https://HOST/realms/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5/broker/ALIAS/endpoint`, replacing `HOST` with `hosts.voting` and `ALIAS` with `idp_alias` from your working `site.json` |
| Redirect URI of the customer's tenant | The exact URI printed by `./deploy commission site.json` after customer creation; copy that complete URI into the provider registration |
| Redirect URI of each election event | Its exact event-realm URI, registered when the event is created; follow [Self-Test and Handover](11-selftest_handover.md) |
| Claims | the user name and e-mail address of each person, as their account in the TOE will carry them; the actual authentication time in the signed ID token (`auth_time`) |
| Authentication | two factors for administrators and election board members |
| Fresh authentication | honour `prompt=login` with a fresh login, including both factors for a board member, even when an upstream session exists; honour `max_age` and return the actual authentication time, not the token issue time |

:::warning
Register exact redirect URIs only, never a wildcard. The authentication
server must refuse every other redirect URI for this client.
:::

At customer commissioning, the terminal prints the complete customer URI and
pauses while the provider's operator registers it. Type `CONTINUE` only after
that registration and the customer administrator account are ready.

![Actual commissioning terminal showing the exact customer redirect URI and registration checkpoint](./assets/installation/installer-commission-complete.png)

*Actual VM rehearsal capture. Use the URI printed by your own commissioning run.
The provider's registration screen depends on its product; this screenshot shows
the TOE command's required value and its explicit checkpoint.*

**If registration cannot be completed:** leave the command paused. Resolve the
exact URI, client settings and administrator identity with the provider's
operator. Do not register a wildcard or continue on the assumption that the
provider will accept the login.

From the operator of the authentication server, obtain, through a channel
that protects their integrity: the issuer, the authorization endpoint, the
token endpoint, the URL of its signing keys (JWKS), the end-session endpoint,
the client ID and the client secret of the TOE's registration, and the CA
certificate its server certificate chains to, with the SHA-256 fingerprint of
that CA certificate.

## Write the provider definition

:::warning
The provider definition holds the client secret. Keep the input in a root-only
protected directory on the administration workstation; the installer transfers
it to C1 over the verified administration connection.
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

**Check it from a root terminal in the administration workstation's `deployment`
directory**, after the full working configuration and server installations are ready:

```bash
./deploy check site.json
```

This is the same [site preflight](01-about.md#check-the-site), not an additional
installation command. It checks the definition's issuer, HTTPS URLs, client ID,
signature-validation settings and first-login flow, as well as the CA fingerprint.

**Expected result:** `PASS: configuration, accepted media and certificate inputs`,
then the node checks and `CHECK PASSED — ready to install the platform`.
The client secret is not printed.

![Actual preflight validating protected certificate and provider inputs before installation](./assets/installation/installer-preflight.png)

*Actual rehearsal terminal. The first PASS line covers the input validation;
a successful file check does not prove that the external provider has completed
registration or that a user can authenticate with both factors.*

**If it differs:** compare the public values with the independently supplied
registration, correct the private file in the editor and rerun the check. Never
paste unredacted contents into a report or put the client secret in `site.json`.


Check the public registration values against those you received. `validateSignature` and
`useJwksUrl` make the TOE accept only ID tokens signed with the server's
published keys, and `issuer` makes it accept only tokens of that issuer. The
first-login flow `certificate-first-login-flow` links a sign-in to an existing
account and never creates one.

## Install the trust anchor

<!-- deployment-step: 06-002 -->

Set `identity_provider.ca` to the independently supplied CA file and
`identity_provider.ca_sha256` to its separately received fingerprint. The
preflight above compares the actual fingerprint before installation.

**Automatic part of `./deploy install site.json`:** the installer installs the
checked trust anchor and configures Keycloak to use it with the required TLS
profile. No manual ConfigMap command is needed. The [installation instructions
and actual terminal capture](01-about.md#install-the-platform) show the command
and the final `INSTALL 4/4` completion message.

**If a fingerprint or trust-anchor installation check fails:** stop. Do not copy
an observed fingerprint into the expected field, replace an existing ConfigMap
blindly or disable certificate verification. Reconcile the CA and partial
installation state with the operator before proceeding.

## Check the authentication server's TLS

<!-- deployment-step: 06-003 -->
<!-- deployment-step: 06-004 -->

**Where:** a root terminal in the administration workstation's `deployment`
directory, after the platform has been installed. Run the same live TLS inspection
used in [Certificates and TLS](05-certificates_tls.md#check-the-tls-profile):

```bash
./deploy inspect site.json tls
```

The command checks the TOE's endpoints and makes full authenticated TLS probes
from **both C1 and C2** to the configured authentication host, because Keycloak
can run on either node. The public, fingerprint-verified CA certificate is
provided to each inspection through a temporary file and removed afterwards.
No client secret or private key is sent by these probes.

![Actual TLS inspection including connections from both servers to the authentication service](./assets/installation/inspect-tls.png)

**Expected result:** the lines labelled `C1 to authentication server` and
`C2 to authentication server` each show a verified TLS 1.3 connection with
`TLS_AES_256_GCM_SHA384`, `secp384r1` and a 384-bit server key. TLS 1.2, AES-128,
ChaCha20 and X25519 attempts are refused; the allowed connection still succeeds
after those rejection tests. The run ends with
`TLS PROFILE CHECK PASSED — allowed connections verified and prohibited variants refused`.

**If either server fails:** check that server's DNS, outbound TCP-443 route,
configured host name and verified CA. If a prohibited variant is accepted,
have the authentication-server operator correct its profile. A timeout is an
incomplete check, not proof that a prohibited protocol was refused. Do not change
the host to an unrelated reachable service or disable name/chain verification.

[Configuring the Deployment](07-hardening.md) explains the settings already
applied by installation. Continue with [customer and trustee commissioning](08-installing_toe.md#create-the-customers-tenant)
if it is still pending. Successful transport checks still require the browser
login, MFA and fresh-authentication checks before handover.
