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

> **Note:** The extension requires Keycloak **26.5.5** or later. It references
> `UserAuthenticationIdentityProvider` which was introduced in that version.

---

## Step 1 — Set Up the Local Wallet

The [oid4vc-dev](https://github.com/dominikschlosser/oid4vc-dev) tool ships a
lightweight browser-based wallet with pre-loaded sample PID credentials, suitable
for local OID4VP testing without a real mobile wallet.

Run all wallet commands from a **dev container terminal** (VS Code integrated terminal).

### Install oid4vc-dev

`oid4vc-dev` is not bundled in the dev container — install it once with:

```bash
ARCH=$(uname -m); OS=$(uname -s | tr '[:upper:]' '[:lower:]')
case "$ARCH" in x86_64) ARCH="amd64" ;; aarch64|arm64) ARCH="arm64" ;; esac
VERSION=$(curl -fsSL https://api.github.com/repos/dominikschlosser/oid4vc-dev/releases/latest | grep '"tag_name"' | cut -d'"' -f4)
sudo curl -fsSL -o /usr/local/bin/oid4vc-dev \
  "https://github.com/dominikschlosser/oid4vc-dev/releases/download/${VERSION}/oid4vc-dev-${VERSION}-${OS}-${ARCH}"
sudo chmod +x /usr/local/bin/oid4vc-dev
oid4vc-dev --help
```

This downloads the latest release binary directly from GitHub for your platform (Linux amd64 or arm64).

### Bridge port 8090 to Keycloak

Inside the dev container, `127.0.0.1:8090` is the container's own loopback — it
does not reach Keycloak. Use `socat` to proxy it to Keycloak's Docker network IP:

```bash
# Find Keycloak's Docker network IP
docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' keycloak
# e.g. 172.18.0.2

socat TCP-LISTEN:8090,fork,reuseaddr TCP:<KC-IP>:8090
```

Leave this running in its own terminal tab.

### Start the wallet

```bash
oid4vc-dev wallet serve --pid --port 8089
```

- `--pid` generates fresh PID credentials on startup (without a revocation status
  list, so Keycloak will not attempt a revocation check).

VS Code detects port 8089 and offers to forward it — accept, so the wallet UI is
accessible from your browser at **http://localhost:8089**.


#### Check wallet.json for port conflicts

After starting the wallet, verify that the `issuer_url` in `wallet.json` does not
use a port already occupied by another service:

```bash
tail /home/vscode/.oid4vc-dev/wallet/wallet.json
```

Look for the `"issuer_url"` field (e.g. `"https://localhost:8086"`). If that port is
in use, edit the file and change it to a free port. This URL is embedded in generated
credentials and Keycloak will attempt to connect to it when checking revocation status.


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
| **Trust List URL** | `http://<devcontainer-ip>:8089/api/trustlists/pid` (see note below) |
| **Trust List Signing Certificate** | *(leave empty — acceptable for local testing)* |
| **User Identifier Claim** | `personal_administrative_number` |
| **Same-device flow** | enabled |
| **Cross-device flow** | enabled |
| **Request object lifespan** | `10` |

> **Finding the devcontainer IP** — Keycloak cannot use `localhost` to reach the
> wallet (that resolves to Keycloak's own loopback). Use the devcontainer's Docker
> network IP instead:
> ```bash
> docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' devcontainer
> # e.g. 172.18.0.13
> ```
> The Trust List URL then becomes `http://172.18.0.13:8089/api/trustlists/pid`.
> Note that `/api/trustlists` (without `/pid`) returns an index, not the trust list
> JWT itself — the `/pid` suffix is required.

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

> **Credential Type values** — `urn:eudi:pid:de:1` is the VCT for `dc+sd-jwt`
> format. Do not confuse it with `eu.europa.ec.eudi.pid.1`, which is the doctype
> for `mso_mdoc` format. Using the mDoc doctype with a dc+sd-jwt mapper causes
> the DCQL query to match zero credentials.

> The `oid4vc-dev` PID credential does not contain a `sub` claim. The stable
> unique identifier is `personal_administrative_number` (e.g. `L01X00T47`),
> which corresponds to a national ID number. You can inspect all available claims
> in `/home/vscode/.oid4vc-dev/wallet/wallet.json` under `credentials[0].claims`.

### Matching the credential to a Keycloak user

1. **Create the user attribute in Keycloak** — go to **Realm Settings → User Profile
   → Create attribute** and add `personal_administrative_number`.

2. **Find the credential value** — note the `personal_administrative_number` from
   `wallet.json` (e.g. `L01X00T47`).

3. **Create a new voter** — in the **Admin portal**, add a voter with username `L01X00T47`
    and set the user attribute `personal_administrative_number` = `L01X00T47`.

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
| 2 | `Automatically Set Existing User` | REQUIRED |

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

