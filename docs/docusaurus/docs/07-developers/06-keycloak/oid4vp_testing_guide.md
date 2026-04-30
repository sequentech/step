---
id: oid4vp_testing_guide
title: OID4VP Testing Guide
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# OID4VP Testing Guide

This guide covers how to test the [keycloak-extension-oid4vp](https://github.com/ba-itsys/keycloak-extension-oid4vp)
(v0.6.2) that is bundled into the Keycloak image via `packages/Dockerfile.keycloak`.

> **Note:** The extension requires Keycloak **26.6.1** or later. It references
> `UserAuthenticationIdentityProvider` which was introduced in that version.

---

## Step 1 — Set Up the Local Wallet

The [oid4vc-dev](https://github.com/dominikschlosser/oid4vc-dev) tool ships a
lightweight browser-based wallet with pre-loaded sample PID credentials, suitable
for local OID4VP testing without a real mobile wallet.

> **Run the wallet on your local machine (Windows, macOS, Linux), not inside the
> dev container.** The wallet needs to share the same network as your browser so
> that the `openid4vp://` request URI — which contains `127.0.0.1:8090` pointing
> to Keycloak — is reachable when the wallet fetches it. Running inside the dev
> container breaks this because `127.0.0.1` inside the container does not reach
> Keycloak.

### Install `oid4vc-dev`

Download the prebuilt binary for your platform from the
[releases page](https://github.com/dominikschlosser/oid4vc-dev/releases/tag/v1.9.4):

| Platform | Binary |
|---|---|
| Windows | `oid4vc-dev-v1.9.4-windows-amd64.exe` |
| macOS (Apple Silicon) | `oid4vc-dev-v1.9.4-darwin-arm64` |
| macOS (Intel) | `oid4vc-dev-v1.9.4-darwin-amd64` |
| Linux x86\_64 | `oid4vc-dev-v1.9.4-linux-amd64` |

### Start the wallet

Generate sample PID credentials into it:

```bash
oid4vc-dev wallet generate-pid
```

Then start the wallet:

```bash
oid4vc-dev wallet serve --port 8089
```

The wallet UI is accessible at **http://localhost:8089**.

The startup output prints the trust list URLs — look for the `Trust Lists:` line:

```
Trust Lists:  http://localhost:8089/api/trustlists
              https://localhost:8090/api/trustlists
```

When entering the URL in Keycloak (Step 2), replace `localhost` with
`host.docker.internal` so the Keycloak Docker container can reach the wallet
running on the host machine:

```
http://host.docker.internal:8089/api/trustlists
```

> On Linux Docker, `host.docker.internal` may not resolve. If it doesn't, find
> the gateway IP and substitute it:
> ```bash
> docker network inspect bridge --format '{{range .IPAM.Config}}{{.Gateway}}{{end}}'
> ```

---

## Step 2 — Configure the Keycloak Realm

Go to **http://127.0.0.1:8090** → Admin Console (admin/admin) → pick your target realm.

Navigate to **Identity Providers → Add provider → OID4VP**.

### General settings

| Field | Value |
|---|---|
| **Client ID** | `voting-portal` |
| **Client authentication** | `none` |
| **Response mode** | `direct_post` |
| **URI scheme** | `openid4vp://` |
| **Client ID Scheme** | `plain` |
| **Trust List URL** | `http://host.docker.internal:8089/api/trustlists` (see note below) |
| **User Identifier Claim** | `personal_administrative_number` |
| **Same-device flow** | enabled |
| **Cross-device flow** | enabled |
| **Request object lifespan** | `10` |

> The Trust List URL is printed by `oid4vc-dev wallet serve` on startup under
> `Trust Lists:`. Use the `host.docker.internal` variant of that URL (replacing
> `localhost`) so the Keycloak Docker container can reach the wallet running on
> the host.

### Mappers

Go to the **Mappers** tab on the provider page and click **Add mapper**:

| Field | Value |
|---|---|
| **Mapper type** | `OID4VP Claim to User Attribute` |
| **Credential format** | `dc+sd-jwt` |
| **Claim path** | `personal_administrative_number` |
| **User attribute** | `personal_administrative_number` |
| **Credential Type** | `urn:eudi:pid:de:1` |

The mapper serves a dual purpose: it maps the credential claim onto a Keycloak
user attribute **and** drives automatic DCQL query generation — the extension
uses the mapper definitions to build the credential request sent to the wallet,
so no manual DCQL query is needed for basic testing.

> The `oid4vc-dev` PID credential does not contain a `sub` claim. The stable
> unique identifier is `personal_administrative_number` (e.g. `L01X00T47`),
> which corresponds to a national ID number. You can inspect all available claims
> in `/home/vscode/.oid4vc-dev/wallet/wallet.json` under `credentials[0].claims`.

### Matching the credential to a Keycloak user

1. **Create the user attribute in Keycloak** — go to **Realm Settings → User Profile
   → Create attribute** and add `personal_administrative_number`.

2. **Find the credential value** — note the `personal_administrative_number` from
   `wallet.json` (e.g. `L01X00T47`).

3. **Set the attribute on an existing voter** — in the **Admin portal**, open the
   voter record and set `personal_administrative_number` = `L01X00T47`.

4. On next login via the wallet, Keycloak will match on that attribute and link
   the session to that voter.

---

## Step 3 — Wire the Authentication Flow

Unlike the `digital-certificates` IDP (which uses a self-federation trick to
switch to a mutual-TLS port), the OID4VP IDP communicates via HTTP callbacks and
does not need a separate client. The `voting-portal` client already uses the
default browser flow, which includes an `identity-provider-redirector` step —
this is what surfaces IDP buttons on the login page. The OID4VP button will appear
there automatically once the IDP is enabled.

What does need wiring is the **first broker login flow** — the flow Keycloak runs
after the wallet authentication succeeds, to link the wallet identity to an
existing voter.

### Create the first broker login flow

Go to **Authentication → Flows → Create flow** and create a new top-level flow
named `oid4vp-first-login-flow`. Add two steps in order:

| Step | Authenticator | Requirement |
|---|---|---|
| 1 | `Detect Existing Broker User` (`idp-detect-existing-broker-user`) | REQUIRED |
| 2 | `Automatically Link Brokered Account` (`idp-auto-link`) | REQUIRED |

Step 1 looks up a Keycloak user whose `personal_administrative_number` attribute
matches the value from the wallet credential. Step 2 links that user's account to
the wallet identity so future logins are recognised automatically.

### Assign the flow to the OID4VP IdP

Back on the OID4VP Identity Provider config page, set:

| Field | Value |
|---|---|
| **First Broker Login Flow** | `oid4vp-first-login-flow` |

### Control button visibility in login.ftl

The voting portal login template
(`keycloak-extensions/sequent-theme/src/main/resources/theme/sequent.voting-portal/login/login.ftl`)
already filters `social.providers`. Following the same pattern as
`digital-certificates` (controlled by the `voter-certificate-policy` realm
attribute), add a filter for the OID4VP button keyed on a new realm attribute
(e.g. `voter-oid4vp-policy`). Until then the button will appear unconditionally.

---

## Step 4 — Test the Flow

1. Open the voting portal at **http://127.0.0.1:3000** and trigger a login.

2. You should see an OID4VP button on the login page alongside the existing
   identity providers.

3. Click the OID4VP button. A QR code and an **"Open Wallet App"** button appear.

4. **Right-click the "Open Wallet App" button → Copy link address** to get the
   full `openid4vp://?client_id=...&request_uri=...` URL.

5. Open the wallet UI at **http://localhost:8089**, paste the URL into the
   **"Paste OID4VCI offer URI or OID4VC request URI"** field and submit.

6. The wallet fetches the credential request from Keycloak, shows the claims
   being requested, and asks for consent. Approve it to complete the login.


---

