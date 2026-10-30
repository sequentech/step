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
edits show up live), `--logs DIR`, `--version V`, `--election ID`.

| Origin | What |
|---|---|
| `http://127.0.0.1:18850` | voting portal (the preview's own `/global-settings.json`, `/v1/graphql` forwarded to the stand-in, the rest from webpack, or a static build). The portal sends GraphQL to its own origin because the Vitrine extension reads casts only there, as a reverse proxy in front of Hasura serves them in a deployment the extension checks |
| `http://127.0.0.1:18853` | admin portal |
| `http://127.0.0.1:18851` | the stand-in: GraphQL (`/v1/graphql`), a fake Keycloak (`/realms/...`), the published ballot files, `/__preview/status` |
| 18852, 18854 | the webpack dev servers (internal) |

Voter entry: `http://127.0.0.1:18850/tenant/90505c8a-23a9-4cdf-a26b-4e19f6a097d5/event/demo-2026/login`
(signs in the fixture voter without a password). Admin: `http://127.0.0.1:18853/`, then the
event's Transparency tab. The container `vitrine-ui-harness-relay` (the only one started) shares the
stack's network namespace to reach the log server's internal listener through a Unix socket.

## What is live and what is stubbed

Live: the policy (the stack's sealed policy, read from `deploy/.state/public`), the ballots log
(every cast is submitted to it through its `add` route with its submission token and answered with
its real receipt or promise), every read of the receipt chain (the portal reads the log, witnesses,
hubs and monitors directly), the events log (the booth's config statement and `voting-opened`
milestone are served from it), and the admin's Transparency tab (the sidecar's `status` and
`GET /admin/logs`).

Stubbed: everything else Step's backend does. The election, ballot style, area and voter are
fixtures (`lib/fixtures.mjs`); the Keycloak issues unsigned development tokens (`alg: none`, which
keycloak-js doesn't verify); `get_ballot_files_urls` serves the fixtures as JSON files; tallying,
users, reports and other admin tabs answer empty. The voter's ballots are the ones cast during
the session: `get_voter_cast_votes` finds them by Tracking ID or by the prefix of a short ID, with
their cast time, so the voting portal's ballot tracker (Ballot Tracking) works end to end.

It also holds one audited ballot, the JSON the portal's Audit screen shows for the fixture election
(`lib/audited-ballot.mjs`, short ID `9020 e384`). At start-up it records the ballot's ID in the
ballots log, as a cast would be, and a tracker lookup that names that ID gets the ballot back, so
the tracker's audit view (the checks, then the decoded selections) can be shown. Step's backend does
not publish audited ballots: this is the stand-in's own addition, and it never shows in the ballot
list.

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

## Playwright

`playwright/run.sh NAME SCRIPT OUTDIR [args]` runs a script in `mcr.microsoft.com/playwright:v1.64.1-noble`
on the host network, with playwright-core from `$VITRINE_DIR/scripts/video/node_modules`.
`voter.mjs` (cast and watch the receipt), `admin.mjs` (the Transparency tab) and `explore.mjs`
(what a page shows, with `TRACE=1` for the stand-in's requests) are the recipes.
