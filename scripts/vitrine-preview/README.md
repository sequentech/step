<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Vitrine preview of Step's portals

Runs Step's real voting portal and admin portal in a browser against the live
local Vitrine stack (`deploy/up.sh` in the Vitrine repository), without a Step
deployment. For screenshots and recorded demos only: it is not a Step backend.

```
node scripts/vitrine-preview/preview.mjs start [--wait]   # everything; returns once serving
node scripts/vitrine-preview/preview.mjs stop             # only what start started
node scripts/vitrine-preview/preview.mjs status
```

Node 20, no dependencies. Flags: `--vitrine DIR` (default `~/vitrine`), `--step DIR`
(default `~/step-vitrine`, the Step checkout whose webpack dev servers are run, so its
edits show up live), `--logs DIR`, `--version V`, `--election ID`, `--no-verifier` (don't serve hub-b's
ballot verifier).

| Origin | What |
|---|---|
| `http://127.0.0.1:18850` | voting portal (the preview's own `/global-settings.json`, `/v1/graphql` forwarded to the stand-in, the rest from webpack, or a static build). The portal sends GraphQL to its own origin because the Vitrine extension reads casts only there, as a reverse proxy in front of Hasura serves them in a deployment the extension checks |
| `http://127.0.0.1:18853` | admin portal |
| `http://127.0.0.1:18856` | hub-b's ballot verifier: Step's ballot-verifier app (see "hub-b's ballot verifier"), `/v1/graphql` forwarded to the stand-in |
| `http://127.0.0.1:18851` | the stand-in: GraphQL (`/v1/graphql`), a fake Keycloak (`/realms/...`), the published ballot files, `/__preview/status` |
| 18852, 18854, 18857 | the webpack dev servers (internal) |

Voter entry: `http://127.0.0.1:18850/tenant/90505c8a-23a9-4cdf-a26b-4e19f6a097d5/event/demo-2026/login`
(signs in the fixture voter without a password). Admin: `http://127.0.0.1:18853/`, then the
event's Transparency tab. The container `vitrine-ui-harness-relay` (the only one started) shares the
stack's network namespace to reach the log server's internal listener through a Unix socket.

## What is live and what is stubbed

Live: the policy (the stack's sealed policy, read from `deploy/.state/public`), the ballots log
(every stage of a ballot is submitted to it through its `add` route with its submission token and
answered with its real receipt or promise), every read of the receipt chain (the portal reads the log, witnesses,
hubs and monitors directly), the events log (the booth's config statement and `voting-opened`
milestone are served from it), and the admin's Transparency tab (the sidecar's `status` and
`GET /admin/logs`).

Stubbed: everything else Step's backend does. The election, ballot style, area and voter are
fixtures (`lib/fixtures.mjs`); the Keycloak issues unsigned development tokens (`alg: none`, which
keycloak-js doesn't verify); `get_ballot_files_urls` serves the fixtures as JSON files; tallying,
users, reports and other admin tabs answer empty. The voter's ballots are the ones cast during
the session.

## The ballot box (`lib/ballot-box.mjs`)

The event's receipts policy is `signed-by-ballot-box` with unlimited checks, so the portal sends the
voter-signed ballot to the ballot box when the review screen opens, and casts it from there. The
stand-in plays that ballot box as Harvest and Windmill do:

| Mutation | What it does |
|---|---|
| `receive_ballot` | checks that the content parses, carries the voter's key and signature and that the signature covers the ballot hash, the election and the ballot; signs the Received statement with the stand-in's Ed25519 ballot box key (`receipt key` `XXXX-XXXX`); writes the `received` entry (Tracking ID digest) to the ballots log. The same ballot again answers the same receipt and writes nothing |
| `cast_ballot` | checks the voter's signature over the receipt key (the Cast statement), records the cast, writes the `cast` entry, signs the Cast receipt. The same request again returns the same receipt with no log answer, as Harvest does. An audited ballot, a ballot already cast with another signature, a ballot never received or a signature that doesn't verify answer 400 `PokValidationFailed` |
| `audit_ballot` | marks the received ballot audited and writes the `audited` entry; a cast ballot is refused (409), an unknown one is not found. With `auditable_ballot` (the portal sends it in a second call, once the box has confirmed the audit) it publishes that ballot for the public locator, if its ballot hash and key are the audited ballot's; `publication` answers `published` or `not-published`, and a published ballot stays as first published |
| `insert_cast_vote` | the direct path: receives the ballot, waits for the log to sequence the `received` entry (the profile's received-first rule), then writes `cast`. It has no cast signature, so it signs no Cast receipt |
| `get_voter_cast_votes`, `locate_ballot` | the voter's cast ballots, found by the full Tracking ID, the start of one (8 or more Crockford characters, ignoring case and hyphens, with I and L read as 1 and O as 0) or the ballot's hex hash. `locate_ballot` answers found, not-found or ambiguous (listing the Tracking IDs when there are ten or fewer). With no token (a hub's ballot verifier) it is a public locator: it finds only the published audited ballots, the fixture's and those the portal published, whoever's they are, and never a cast ballot |
| `ballot_box_own` | the voter's own ballots at every stage, newest first, as the ballot box knows them by the voter it received them from; the ballot list shows the check mark of the cast one from it |

The ballot box key is published in the ballot style (`ballot_eml.ballot_box_key`), the only place the
portal reads it from, and its seed is fixed (`BALLOT_BOX_SEED` in `lib/fixtures.mjs`). This makes
the key part of the style text the events log's config statement commits to, so the events log must be
provisioned from the fixtures of this checkout (see below); a log provisioned from older fixtures
makes the booth check refuse. The same goes for the ballot style's `keys_ceremony_id`, which the
portal's encryption needs.

Skipped: `ballot_box_log`, `ballot_box_own`, `ballot_box_entry` and `ballot_box_keys` (the dialog of
the ballot box log, behind the logs tab that the fixture hides with `hide-logs-tab`); the
ended-checks period (the checks never end here); recomputing the ballot hash from the content (the
portal's ballots are version 3, whose hash binds the ballot style, so the stand-in takes the hash
the portal sends and checks only the voter's signature over it).

Every stage of a ballot is a `vitrine/election/ballots/v1` entry (`received`, then `cast` or
`audited`) naming the ballot by its Tracking ID digest, so a ballot that is not received first is
refused by the log's profile. The Tracking ID the voter reads is the first 8 characters of that digest
in Crockford base 32 (`BJHF-GCZX`).

It also holds one audited ballot, the JSON the portal's Audit screen shows for the fixture election
(`lib/audited-ballot.mjs`). At start-up it writes the ballot's `received` and `audited` entries to the
ballots log under its Tracking ID (`/__preview/status` names it, `ballotBox.auditedFixture`), and a
tracker lookup that names that ID gets the ballot back, so the tracker's audit view (the checks, then
the decoded selections) can be shown. Step's backend publishes only ballots a voter audited: the
fixture is the stand-in's own addition, and it never shows in the ballot list.

`node --test scripts/vitrine-preview/lib/` runs the unit tests: the statement and Tracking ID
vectors of sequent-core and the ballot box against a fake log.

## The booth check needs the events log provisioned

Before encrypting, the portal checks the election's `vitrine/config/v1` statement and the event's
`voting-opened` milestone in the events log (Step's `booth check`). The events log takes one
`voting-opened`, and the stack's demo booth writes it at start-up for its own election (`mayor`),
key, styles and release, so a stack as `deploy/up.sh` leaves it makes the portal refuse ("Your ballot
can't be prepared yet", reason `opened-config-mismatch`: the real refusal, not a harness fault).
`provision-events` writes what Step's election needs (`--dry-run` shows it):

```
node scripts/vitrine-preview/preview.mjs provision-events --with-demo-booth [--release-version V]
```

It must run on a fresh events log before the demo booth sets itself up (stop the booth container, run
it, start the booth: the booth adopts a milestone that names its election too, which
`--with-demo-booth` arranges). It refuses, and writes nothing, on a log whose milestone doesn't name
Step's election. The config statement commits to the election key and ballot style of the fixtures
and to `release sequent/voting-portal <V>`, so `<V>` must be the version the portal build reports
(`APP_VERSION` in the served settings: `--version`, or the release bundle's version).

## Static mode (a production build with SRI)

```
node scripts/vitrine-preview/preview.mjs start --static-dist DIR --release-bundle FILE [--release-package P --release-version V]
```

The voting origin then serves `DIR` (a `webpack --mode production` build) with the dev server's
headers (COOP same-origin, COEP require-corp, CORP cross-origin: the portals' multi-threaded WASM
needs cross-origin isolation), the single-page app's fallback, and `FILE` at
`/.well-known/vitrine/release/<package>/<version>` (package and version read from the file if it is
JSON naming them). `/global-settings.json` is the preview's: `preview.mjs settings voting` prints
the exact bytes, for a release bundle that covers it.

## The Alert scenario

`node scripts/vitrine-preview/preview.mjs tamper on|off` switches a dishonest operator on. With
the ballots log's own key it signs a checkpoint of the size of the voter's receipt and a root
without the voter's ballot. Witnesses never signed it and the real log doesn't serve it: the
browser is shown it by `playwright/tamper-route.mjs` (`installTamperRoute(context)` before
navigating), because the portal reads the log's URL from the pinned policy. The portal's own
checks then hold two log-signed checkpoints of one size with different roots (split-view evidence).

## The staged attack, version 2: a dishonest portal

`node scripts/vitrine-preview/preview.mjs dishonest-portal on|off` stands in for an operator that
also controls the voting portal. With it on, the voting portal's origin serves its pages with one
more script in front of the app's own (`lib/dishonest-portal.mjs`), which answers every hub's
`GET <hub>/problems` with an empty list, so the voter's own page goes on saying the ballot is
verified. Nothing in Step's code changes. The split view itself comes from Vitrine's
`deploy/split-view.sh`, which gossips a second checkpoint of the voter's receipt's size to hub-b:
hub-b holds the voter's checkpoint from their confirmed watch made from the receipt, proves the
split view, records it and mails the watchers. The voter learns of it from what the operator
doesn't serve: the hub's email and site, a monitor, the browser extension. The admin portal is
not affected, so its banner shows it too.

`POST http://127.0.0.1:18851/__preview/split-view` (`{}`, or `{"dryRun": true}`) stages that split view
for a recording: it fetches the log's receipt for the voter's latest cast, saves it, runs
`$VITRINE_DIR/deploy/split-view.sh --receipt <file>` on this machine and answers `{ok, code, output}`
(the script's output, with the `409 evidence sha256:…` line and the addresses of the problem pages). It answers
only a loopback peer asking a loopback Host, answers 409 before anything is cast, and runs one at a time.
The recorder runs in Docker and can't run host scripts, so a board's `fetch` step calls this instead.

`POST http://127.0.0.1:18851/__preview/audited-ballot` (`{"content": "<the auditable ballot's JSON>"}`, loopback
only) publishes the auditable ballot of a ballot the voter has audited, as `audit_ballot` does when the portal
sends it. Nothing needs it now that the portal publishes the ballot itself; it stays for older boards, which copy
the JSON from an Audit screen with no transparency log (`.auditable-ballot-data`). It answers 422 for JSON that
isn't the voter's audited ballot (matched by its ballot hash and key).

## hub-b's ballot verifier

`start` also runs Step's ballot-verifier app from the `--step` checkout (webpack dev server on 18857)
behind a front on `http://127.0.0.1:18856/`, as the ballot verifier that hub-b runs for audited ballots
(`preview.mjs settings verifier` prints its settings). The front serves its `/global-settings.json`
(`HUB_NAME` `hub-b`, no sign-in, `VOTING_PORTAL_URL` the voting portal for its back button,
`PUBLIC_BUCKET_URL` the stand-in's, whose event config carries the sealed policy) and forwards
`/v1/graphql` to the stand-in, whose `locate_ballot` answers a request with no token with the published
audited ballots only. Everything else it checks (the ballot's bucket on the lookup server, the receipt and
checkpoints of the ballots log, the events log's checkpoint) it reads from the Vitrine stack itself.
The stand-in allows its origin.

The policy names it only if the stack was provisioned with it, a choice sealed with the policy:

```
cd $VITRINE_DIR && deploy/down.sh --purge
VITRINE_HUB_B_BALLOT_VERIFIER=http://127.0.0.1:18856/ deploy/up.sh   # with VITRINE_EXPLORERS=1 for hub-b's site
```

then `provision-events` as above. The policy then has `hub-ballot-verifier hub-b http://127.0.0.1:18856/`,
so the voting portal's audit screen offers hub-b to compare an audited ballot on, and hub-b's site links
audited ballots to it. `start` and `status` say when the policy names no verifier or another one. A link opens
an audited ballot on it:

```
http://127.0.0.1:18856/tenant/<tenant>/event/<event>/election/<election>/audited#id=<full Tracking ID>&policy=sha256:<hex>
```

`status` prints the link of the audited fixture. The verifier needs the audited ballot screen
(packages/ballot-verifier, `AuditedScreen`) in the `--step` checkout.

## Playwright

`playwright/run.sh NAME SCRIPT OUTDIR [args]` runs a script in `mcr.microsoft.com/playwright:v1.64.1-noble`
on the host network, with playwright-core from `$VITRINE_DIR/scripts/video/node_modules`.
`voter.mjs` (cast and watch the receipt), `admin.mjs` (the Transparency tab) and `explore.mjs`
(what a page shows, with `TRACE=1` for the stand-in's requests) are the recipes.
