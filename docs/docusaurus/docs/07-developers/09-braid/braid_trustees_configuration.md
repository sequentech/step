---
id: braid_trustees_configuration
title: Wbraid trustee configuration
sidebar_label: Wbraid trustees
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The native `packages/trustee` runner uses the protocol and cryptography supplied
by `packages/wbraid`. Each trustee keeps a signing key and a share encryption
key locally. The administration portal registers both public keys.

Generate a new identity once, then print its public keys when registering it:

```sh
trustee generate --trustee-config trustee.toml
trustee public-keys --trustee-config trustee.toml
trustee start --trustee-config trustee.toml
```

Generation refuses to replace an existing file. Back up `trustee.toml` and the
runner's `message_store` directory. The runner does not generate replacement
keys when its configured identity is missing.

| Variable | Purpose |
| --- | --- |
| `TRUSTEE_CONFIG_PATH` | Local native keys file, unless supplied with `--trustee-config` |
| `B4_URL` | Native board URL, unless supplied with `--b4-url` |
| `TRUSTEE_HARVEST_URL` | Platform API that lists this trustee's assigned boards |
| `TRUSTEE_NAME`, `TRUSTEE_PASSWORD` | Trustee's Keycloak account |
| `TRUSTEE_KEYCLOAK_URL`, `TRUSTEE_KEYCLOAK_REALM` | Tenant's authentication server and realm |
| `TRUSTEE_KEYCLOAK_CLIENT_ID`, `TRUSTEE_KEYCLOAK_CLIENT_SECRET` | Authentication client; omit the secret for a public client |
| `LOG_LEVEL` | Runner tracing filter |

Set an initial password for each trustee account and complete its required
profile fields and MFA enrollment through the tenant portal before starting
the unattended runner. The account must belong to the trustee group and its
`trustee` attribute must match `TRUSTEE_NAME`.

The runner discovers DKG and tally boards through Harvest, verifies signed
configuration and message lineage, and resumes its local protocol sessions.
Containers use `http://b4v6:3005` for the supplied native board service.

For manual ceremonies, download the public custody request from the portal and
produce a signed proof locally:

```sh
trustee attest --request custody-request.json --trustee-config trustee.toml --output custody-proof.json
```

Upload the proof. For a tally restore, add `--tally-session-id <tally UUID>`.
The proof is bound to the event, ceremony, trustee and, for restore, tally.
Private keys stay in the runner's keys file.

The ballot context is the signed configuration's immutable decimal identifier.
Export/import can remap platform UUIDs and board names while preserving signed
canonical messages and that cryptographic identifier.


## Browser trustees

The administration portal's `/trustee` screen runs the supplied wbraid protocol
in WebAssembly. Sign in with the trustee account, then choose **Generate browser
keys**, or **Import private keys** to use an existing native `trustee.toml`.
Legacy browser configurations and protocol stores use incompatible crypto; generate
a new native identity and run a new key ceremony for those elections.
Download and privately back up the keys. Register both displayed public keys for
this trustee before starting its key ceremony. The account's `trustee` attribute
must match the registered trustee name.

Set the browser-reachable board service URL and select **Start browser trustee**.
The page discovers only boards assigned to the authenticated trustee. It drives
key generation and, later, mixing and decryption over each tally's DKG parent.
Keep the page open during participation. **Stop browser trustee** stops polling;
**Lock keys** removes the identity from the page's memory. A page reload requires
importing the keys again. Private keys are never sent to Harvest or the board.

Session commitments and staged-message records persist in IndexedDB, scoped to
both the board service and signing identity. Reloading the same browser resumes
these records. **Back up browser sessions** exports the keys and all recorded
sessions together; keep this JSON file private. **Restore browser backup** restores
them into a fresh browser. A keys file alone is insufficient to restore a tally
participant in a different browser: the recorded DKG commitments must survive.
A Web Lock prevents two tabs in the same browser from running the same identity.
Do not clear browser storage or run the same identity in another browser or native
runner during a ceremony. Native SQLite stores and browser IndexedDB stores are distinct; a
native keys file can enroll a browser participant before DKG, but moving an active
native participant to a browser requires its protocol state as well.

For a manual ceremony, download the public custody request from its wizard. On
the trustee screen, select **Create custody proof** and choose that request.
Upload the downloaded `custody-proof.json` to the wizard. For a tally restore,
enter the tally session UUID before creating the proof. The proof contains only
public metadata and a signature, and the platform checks its ceremony and tally
scope. The browser rejects requests for another trustee or either wrong public key.

Production browser participation requires HTTPS, WebAssembly threads, IndexedDB,
the Web Locks API, and cross-origin isolation. The admin image serves COOP `same-origin` and COEP
`require-corp` headers on documents and worker assets; preserve them at the ingress.
The board service and its presigned S3 URLs must be reachable by the browser, use
HTTPS in production, and permit the portal origin through CORS. Configure:

| Setting | Purpose |
| --- | --- |
| Admin `WBRAID_BOARD_URL` | Browser-reachable native board URL; Compose reads `WBRAID_BOARD_PUBLIC_URL` |
| Admin `HARVEST_URL` | Browser-reachable Harvest API URL |
| Harvest `HARVEST_TRUSTEE_ALLOWED_ORIGINS` | Comma-separated exact portal origins, including scheme and port, without trailing slashes; defaults to `ADMIN_PORTAL_URL` when unset |

Harvest permits preflight only for the trustee board endpoints; actual requests
still require the trustee's JWT and ceremony permission. It does not grant CORS
access to unrelated origins or other APIs.

## Building and validating browser trustees

From the checkout root, use the pinned devenv toolchain:

```sh
devenv shell -- bash -c 'packages/protocol-board/scripts/build-browser.sh'
devenv shell -- bash -c 'cd packages && yarn install'
```

The build uses wbraid's atomics configuration and this checkout's
`packages/rust-local-target/browser-trustee` directory. It packs the bindings and
worker modules into `admin-portal/rust/wbraid-trustee-0.1.0.tgz`; commit the rebuilt
package and Yarn lockfile with source changes. Portal postinstall copies it to
`public/wbraid-trustee`, preserving worker module URLs for node-only frontend
images. Ordinary portal builds consume this vendored package.

```sh
scripts/e2e/run-browser-trustees.sh
```

This disposable journey stops its own native trustees and runs two real Chromium
participants. It verifies manual DKG, browser custody proofs, reload from local
session records, restoration into a fresh browser, a cast vote, and a complete
native tally with an independently checked result. Reports are in
`.cache/backend-e2e/<project>/browser-journey.json`. `--skip-images --skip-build`
reuses prebuilt backend fixtures; browser bindings must already match the source.
The runner cleans up only its own stack and controller.
