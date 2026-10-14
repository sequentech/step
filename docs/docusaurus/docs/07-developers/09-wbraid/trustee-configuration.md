---
id: trustee-configuration
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

| Variable                                                       | Purpose                                                             |
| -------------------------------------------------------------- | ------------------------------------------------------------------- |
| `TRUSTEE_CONFIG_PATH`                                          | Local native keys file, unless supplied with `--trustee-config`     |
| `B4_URL`                                                       | Native board URL, unless supplied with `--b4-url`                   |
| `TRUSTEE_PLATFORM_URL`                                         | Full Hasura GraphQL endpoint; Compose defaults to `HASURA_ENDPOINT` |
| `TRUSTEE_NAME`, `TRUSTEE_PASSWORD`                             | Trustee's Keycloak account                                          |
| `TRUSTEE_KEYCLOAK_URL`, `TRUSTEE_KEYCLOAK_REALM`               | Tenant's authentication server and realm                            |
| `TRUSTEE_KEYCLOAK_CLIENT_ID`, `TRUSTEE_KEYCLOAK_CLIENT_SECRET` | Authentication client; omit the secret for a public client          |
| `LOG_LEVEL`                                                    | Runner tracing filter                                               |

Set an initial password for each trustee account and complete its required
profile fields and MFA enrollment through the tenant portal before starting
the unattended runner. The account must belong to the trustee group and its
`trustee` attribute must match `TRUSTEE_NAME`.

The runner discovers DKG and tally boards through the same public Hasura actions
as browser trustees, verifies signed configuration and message lineage, and
resumes its local protocol sessions. Set `TRUSTEE_PLATFORM_URL` to the full
GraphQL endpoint, including `/v1/graphql`: `http://localhost:8080/v1/graphql`
when running on the development host, or `http://graphql-engine:8080/v1/graphql`
inside the development Compose network. A separately hosted trustee can use the
production public endpoint. The runner sends fixed JSON requests using its
existing HTTP client; it needs no GraphQL client library or schema generation.
Containers use `http://b4v6:3005` for the supplied native board service.

## Development workspace

Both **backend** and **full** development modes start `trustee1` and `trustee2`
alongside `b4v6`, its PostgreSQL database, and LocalStack. The board and trustee
watchers use locked release builds and share this checkout's
`packages/rust-local-target/docker-dev` build cache. Each trustee has a separate
`message_store` volume and uses its identity in `packages/trustee/dev/`.
These checked-in identities are development fixtures; generate private identities
for deployments.

Complete each trustee account's password, profile and MFA setup as described
above. Register **both** public keys from its configured identity in the
administration portal. The runner uses the tenant's `api-key-client` and discovers
only boards assigned to that trustee. A runner can start before enrollment and
retries authentication while the account is being configured.

`scripts/dev/step-dev mode` reads `.env.development` first and `.env` second.
Existing workspaces therefore inherit new settings without replacing local
overrides or secrets. Explicit local values still win: remove an obsolete
override or update it when migrating a workspace. The default
`WBRAID_S3_ENDPOINT_URL=http://localhost.localstack.cloud:4566` reaches LocalStack
through public loopback DNS from a browser and a network alias inside Compose.
No browser-host hosts-file change is needed. Older workspaces with an explicit
`http://localstack:4566` override must update it to this default.
The browser board URL defaults to `http://localhost:3005` through
`WBRAID_BOARD_PUBLIC_URL`.

The full and backend devcontainers forward ports **3002** (admin portal),
**3005** (board), **4566** (storage), **8080** (Hasura) and **8090** (Keycloak).
These ports must retain their local numbers; VS Code reports a conflict instead
of silently selecting another port. Use the local forwarded admin URL,
`http://localhost:3002/trustee`, when developing through a remote workspace.
An HTTPS preview URL also requires HTTPS board and storage endpoints.

### Browser trustee accounts

The default development realm includes `trustee1-wbraid` and `trustee2-wbraid`,
whose `trustee` attributes match their usernames. They belong to the trustee
group and are separate from the native runners' `trustee1` and `trustee2` accounts.
The backend/full post-create setup provisions their initial passwords. To apply
the same setup to an existing realm, run from the checkout root:

```sh
scripts/dev/step-dev browser_trustees
```

When initial password settings are unset or empty, the passwords default to
`trustee1-wbraid` and `trustee2-wbraid`, respectively. Override them with
`BROWSER_TRUSTEE1_PASSWORD` and `BROWSER_TRUSTEE2_PASSWORD` in `.devcontainer/.env`
or the command's environment. Process environment values take precedence over
the file settings. Setup preserves existing passwords, MFA credentials, profile
fields and unrelated attributes. Accounts without MFA receive a synthetic email OTP enrollment marker;
existing MFA methods are preserved. Setup enables message OTP test mode in the
default development tenant because the dummy sender suppresses email content.
Sign in using code **123456** on the email OTP page.
`BROWSER_TRUSTEE_OTP_CODE` overrides this synthetic code. The command leaves
other tenants and the checked-in realm's authentication policy unchanged.

Use a separate browser profile for each account. Generate keys on `/trustee`,
back them up, and register **both public keys** under the corresponding
`trustee1-wbraid` or `trustee2-wbraid` name in **Settings → Trustees** before creating
the ceremony. Select those trustees when creating it. For an existing ceremony,
import its original keys or browser backup instead of generating another identity.
The account's `trustee` claim, rather than its username or loaded public keys,
selects the assigned boards. **Boards assigned to** must show the selected trustee
name; sign out and back in after changing account attributes.

If a configuration was posted but no board appears, verify the account's claim
matches a selected trustee and the ceremony is still **IN_PROGRESS**. Discovery
uses the dashboard's **Platform URL** (`HASURA_URL`); the **Board service URL** is
used after discovery. Click **Refresh boards**, then **Start browser trustee**.

To apply only the crypto service changes to an existing workspace, after checking
its local settings and trustee enrollment, run from the checkout root:

```sh
docker compose --project-directory .devcontainer \
  --env-file .devcontainer/.env.development --env-file .devcontainer/.env \
  -f .devcontainer/docker-compose.yml --profile full up -d --build --no-deps \
  postgres-b4v6 localstack configure-localstack b4v6 trustee1 trustee2
```

This scopes recreation to the listed services and leaves the editor container
running. Start them with existing Hasura, Harvest and Keycloak services available.
Generate a new key ceremony using the registered wbraid identities; legacy
ceremonies and trustee session stores use incompatible protocol data.

For a disposable end-to-end check that does not alter the workspace's services:

```sh
scripts/e2e/run.sh
scripts/e2e/run-browser-trustees.sh
```

The native journeys cover automatic and manual key ceremonies and tallies,
voting, ballot tracking, public verification and election archive import. The
browser journeys exercise both ceremony policies with real Chromium trustees.
Each launcher owns and cleans up its own Compose project and volumes.

## Manual custody proofs

For a server trustee, **Download Ceremony File** in the key-ceremony wizard saves
`<trustee>-custody-request.json`. The operator saves it and uploads that same file
to confirm custody. Upload it again when authorizing a manual tally. The running
trustee fetches the requested work and creates the signed proof automatically;
no shell access or command is needed.

A file upload queues work only. The portal waits for the result, then submits it
through the existing `confirm_key_custody` or `authorize_tally_custody` action.
Additional staff signatures remain required when the event's signing policy
requires them, and bind the hash of the actual proof. Server action results do
not advance ceremony status or consume staff approvals.

The file is public metadata, not a backup of private keys or protocol state. A
proof demonstrates control of the trustee's signing key for the named tenant,
event, key ceremony and optional tally session. It does not demonstrate a backup
exists or that the runner can recover its recorded DKG state. Operators must
still back up both the runner's keys and its private protocol store.

For an offline trustee, produce and upload a signed proof as before:

```sh
trustee attest --request trustee1-custody-request.json --trustee-config trustee.toml --output custody-proof.json
```

For tally authorization add `--tally-session-id <tally UUID>`. Offline and browser
proof files remain accepted directly by both upload steps. Reusing the saved
public file asks the server for a proof of the current tally; reusing an old
signed proof cannot authorize another tally.

### Trustee event updates

Server trustees poll the `trustee_event_updates: jsonb!` Hasura query. Harvest's
`GET /trustee/event-updates` returns assigned `boards` and pending `actions` in
one authenticated response. Each action has an opaque `id`, a typed `kind`, and
its payload. `CREATE_CUSTODY_PROOF` carries the canonical `request`. Native
trustees skip unknown kinds, so more action types can be introduced without
breaking older runners. If the query field is absent during an upgrade, discovery
falls back to `list_trustee_boards`; queued proof work needs the new runner and API.

The portal calls `request_server_custody(object: jsonb!): jsonb!`, forwarded to
`POST /trustee/request-server-custody`. Input names `election_event_id`, exactly
one of `keys_ceremony_id` or `tally_session_id`, and the uploaded
`custody_request_json`. Optional `retry_policy` defaults to `KEEP`;
`RETRY_FAILED` starts a new action after failure. Replies have `status` `PENDING`,
`READY` with `custody_proof_json`, or `FAILED` with `detail`. The first upload
requests a failed-action retry; subsequent polls keep the same action. The portal
stops waiting after two minutes, including stalled network requests, leaving
pending work available for a later upload. It cancels polling and in-flight HTTP
requests when navigating away.

The runner submits `submit_trustee_action_result(result: jsonb!)`, forwarded to
`POST /trustee/action-result`, and requires an explicit `{accepted: true}` before
settling delivery. A result includes the action `id`, canonical `request`, and
`status` `READY` with `custody_proof_json` or `FAILED` with `detail`. Pending work
is replayed until acknowledged. Repeated identical results are accepted; stale
IDs, changed scope, invalid signatures and changed completed results are refused.
Unknown action kinds are left pending for a compatible runner.

All three actions use the `trustee-ceremony` role and trustee JWT claim. Harvest
rebuilds scope from the tenant's stored, signed DKG configuration, checks committee
membership and current ceremony lifecycle, and validates uploaded files against
that scope. The current tally ID is assigned by the platform, never taken from
the saved file. Work and results are stored as JSON strings in existing ceremony
annotations, under `trustee:server-custody:<trustee UUID>`, with a unique action ID
and requesting user. No database migration is needed. A cancelled or inactive
ceremony does not deliver pending work and cannot accept its result. Unreadable
imported action metadata is skipped during discovery so it cannot halt other
boards; a validated current file can replace it with fresh work.

The ballot context is the signed configuration's immutable decimal identifier.
Export/import can remap platform UUIDs and board names while preserving signed
canonical messages and that cryptographic identifier. The same rule applies to
archived tally identifiers: `WasmVerifier.tallyIdentifier` verifies the trusted
signed Ballots statement against its configuration and returns its original
identifier. A copied platform row's UUID is not a cryptographic trust anchor.

## Election event archives

The protocol recovery described here applies to wbraid archives. Pre-wbraid
bulletin-board CSVs contain artifacts from the earlier protocol and cannot be
converted into or resumed as wbraid ceremonies. Keep those original archives
and their matching verifier when recovering or auditing historical ceremonies.

Configuration-only exports can be imported as new events. Publication-only and
activity-log-only archives also work without including native manager secrets
or ceremony state. Older selective archives that inadvertently contain unused
native manager keys remain importable; those keys are ignored. Activity logs
use `sequent-electoral-log-export-v2` to name the main event board explicitly
and preserve independent election audit boards as well as continuation history.
Import verifies every recorded continuation against the earlier archived log's
identity and checkpoint. Legacy linear V1 archives retain their existing checks;
older V1 exports containing several open boards are recognized and recovered.

Select **Bulletin Board** to preserve native DKG configurations, canonical signed
messages, board relationships and the required manager keys. Select **Tally** as
well to preserve tally sessions and results; tally data requires the bulletin
boards. These archives are password encrypted. Import rejects native archives
that omit the manager-key file or any declared DKG board's manager key, or supply
a private manager key that does not match that board's signed configuration.

Import creates a copied election event with new platform IDs. Signed protocol
messages and their original cryptographic configuration identifier remain
unchanged. Ballot publications are unpublished and must be published explicitly.
The copy starts with a fresh ballot box: the archive preserves tally transcripts
and results, rather than restoring accepted ballots, revote history or voter
receipts into a running election.

For a copied manual event, start a tally and use **Download Ceremony File** on
the trustee's tally screen. Save and upload this newly scoped public file. The
original event's file is rejected because its platform IDs differ. Downloading
never queues a proof or changes custody approval; uploading creates a fresh
server proof, followed by the existing human authorization and signing gate.
The same file can be reused for subsequent tallies in the copy. The existing
`get_custody_request` action takes an optional `tally_session_id` for this read-only
download and validates the supplied ceremony against that active tally. Without
it, the initial key ceremony download retains its existing lifecycle checks.

An event archive does not include native trustee private identities or their
local runner state. An imported parent board reuses the trustee's original local
DKG observations by the signed configuration's immutable ID. The runner still
checks the copied transcript against those observations and rejects rewrites or
missing local records. Back up each trustee's private configuration and runner data
separately. Browser trustees retain their existing private-key import/export and
session backup controls; restoring a browser identity requires its original keys.

Reopening a copied DKG ceremony also checks the original local observations and
the predicates of messages already staged by that trustee. An unfinished DKG can
continue when the archive contains those messages. If an original handoff is
missing from the archive, the runner halts before generating another artifact
for that slot; a staged publish handle from the original board cannot be reused
on the copied board. Finish that handoff on the original board and export again.

## Browser trustees

The administration portal's `/trustee` screen runs the supplied wbraid protocol
in WebAssembly. Sign in with the trustee account, then choose **Generate browser
keys**, or **Import private keys** to use an existing native `trustee.toml`.
Legacy browser configurations and protocol stores use incompatible crypto; generate
a new native identity and run a new key ceremony for those elections.
Downloads include the trustee name: for example, `trustee1-private-keys.toml`,
`trustee1-browser-sessions.json` and `trustee1-custody-proof.json`. The ceremony
wizard also prefixes ceremony filenames with the trustee name.
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

For manual key or tally custody, load the original keys, click **Start browser trustee**
and keep `/trustee` open. In another tab, download and upload the
public ceremony file in the ceremony wizard. The browser polls
`trustee_event_updates`, signs the queued request locally with its exact ceremony
and tally scope, and acknowledges it through `submit_trustee_action_result`.
The wizard then verifies the proof and retains the explicit human confirmation
and any required certificate signing. Private keys never leave the browser.

Alternatively, stop the runner, select **Create custody proof** and choose the
public ceremony file. For a tally restore, enter the active tally session's UUID
in **Tally session ID (for tally restore)** before creating the proof. The saved
ceremony file omits that UUID so it can be reused: leaving the field blank creates
a key-ceremony proof, which the tally rejects. Upload the downloaded
`custody-proof.json` to the wizard. The proof contains only
public metadata and a signature, and the platform checks its ceremony and tally
scope. The browser rejects requests for another trustee or either wrong public key.

Production browser participation requires HTTPS, WebAssembly threads, IndexedDB,
the Web Locks API, and cross-origin isolation. The admin image serves COOP `same-origin` and COEP
`require-corp` headers on documents and worker assets; preserve them at the ingress.
The board service and its presigned S3 URLs must be reachable by the browser, use
HTTPS in production, and permit the portal origin through CORS. Configure:

| Setting                  | Purpose                                                                                                                |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------- |
| Admin `WBRAID_BOARD_URL` | Browser-reachable native board URL; Compose reads `WBRAID_BOARD_PUBLIC_URL`                                            |
| Admin `HASURA_URL`       | Public GraphQL endpoint used for assigned-board discovery and halt reports; locally `http://localhost:8080/v1/graphql` |

The displayed **Platform URL** is `HASURA_URL`. Browser trustees use the
`trustee_event_updates` query, `submit_trustee_action_result` mutation and
`report_trustee_board` mutation with their bearer
token and the `trustee-ceremony` role. Hasura forwards those calls internally to
Harvest, which independently verifies the JWT, tenant, trustee identity and
assigned board. Harvest remains private and needs no browser-facing URL or CORS
configuration. Server trustees use `trustee_event_updates` for discovery and pending work, and
`report_trustee_board` for halts, with the same bearer token and role through
`TRUSTEE_PLATFORM_URL`. Both require an explicit successful report
acknowledgement before clearing a pending halt.

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
native tally with an independently checked result. The same browser participants
then complete automatic DKG and tally without manual custody confirmations, with
ballot publication, voting admission, revotes and grace-period checks. Reports are in
`.cache/backend-e2e/<project>/browser-journey.json`. `--skip-images --skip-build`
reuses prebuilt backend fixtures; browser bindings must already match the source.
The runner cleans up only its own stack and controller.

```sh
scripts/e2e/run-server-custody.sh
```

This separate disposable journey keeps the native trustee daemons running and
opens the production administration portal in Chromium. Each trustee logs in,
downloads its public ceremony file, saves two copies, rejects an incorrect file,
and uploads the original to confirm key custody. The same saved file authorizes
a subsequent manual tally without a server command. The journey also checks
that pending custody blocks ballot publication, proofs cannot be replayed across
ceremonies, votes produce the expected result, and an anonymous WASM observer
verifies the public transcripts with invalid-anchor and truncated-transcript
controls. Reports, saved files and screenshots are in
`.cache/backend-e2e/<project>/`. The launcher builds the production portal;
`--skip-ui-build` reuses it, and `--skip-images --skip-build` reuse backend
fixtures. It restores synthetic OTP settings after browser login and cleans up
only its own stack and controller.

[Public ceremony and tally verification](./public-verification.md) describes
the independent native and browser verifier, public exports, trust anchors and
the distinction between ballot tracking and tally-input inclusion.
