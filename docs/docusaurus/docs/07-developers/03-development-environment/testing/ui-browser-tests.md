---
id: ui-browser-tests
title: UI browser tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Storybook runs the UI Essentials and portal stories in Chromium through Vitest.
Use Node 22.22 and Yarn 1.22.22, then from the repository root:

```sh
yarn --cwd packages install --frozen-lockfile
yarn --cwd packages/ui-essentials playwright install chromium
yarn --cwd packages/ui-essentials typecheck:stories
yarn --cwd packages/ui-essentials test:stories --coverage
yarn --cwd packages/ui-essentials build-storybook
```

For the story commands, replace `ui-essentials` with `voting-portal`, `admin-portal`,
`results-portal` or `ballot-verifier`. Semantic checks use `typecheck:stories` for
UI Essentials/admin and `test:types` for the other three portals. `yarn --cwd packages/<package> storybook` opens the interactive
catalog. CI runs interactions, accessibility checks and a catalog build for each
package using the pinned Playwright image. Its JUnit, screenshots and separate
Istanbul coverage reports are uploaded from `test-results/`.

Place typed `*.stories.tsx` beside components, or under their `__stories__`
directory. Use `storybook/test` assertions and spies, query accessible names, and
assert rendered outcomes and callback values. The shared preview supplies the
theme, deterministic English translations and an in-memory router. Configure
`parameters.router` for route parameters and initial history. Screen stories can
provide the real route `action` and its `parentPath` so relative redirects resolve
as they do in the application. The router also accepts a `loader` and
`errorElement` for actual route error boundaries. Browser contexts
use an English locale and UTC. Mock external services; initialize real WASM in a
story loader when the component needs it. Voting ballot stories also reset the
Redux voter session before loading each fixture; see `Question/__stories__` and
`routes/__stories__` for ballot rules, pagination, declaration and decline flows. Tests block unexpected network requests;
only local module, image, font and WASM assets may reach the server.

Admin stories cover event uploads, keys ceremony thresholds and publication controls.
Their provider supplies the production admin theme, tenant and recorded Apollo responses;
assert mutation variables, permission headers, callbacks and visible errors. Run
`yarn --cwd packages/admin-portal typecheck:stories` to check these fixtures and stories.

Stories are excluded from production type builds and the existing Jest coverage
profile. Storybook coverage is reported separately from that gate. Shared test
setup lives in `packages/test-support/storybook/`. Keep changes to production behavior separate from story maintenance.
Known accessibility defects use `parameters.expectedFailure` with a reason and
the exact axe rule IDs. The Vitest hook marks only matching failures as expected;
unrelated failures still fail, and a repaired defect causes an unexpected-pass
failure until its marker is removed. Use `expectedFailure: null` to clear a marker inherited
from the component meta.

The voting journeys exercise the production bundle with local OIDC, GraphQL and
S3 boundaries. Build first, then run:

```sh
yarn --cwd packages build:ui-core
yarn --cwd packages build:ui-essentials
yarn --cwd packages build:voting-portal
yarn --cwd packages/ui-test-kit test
yarn --cwd packages/voting-portal test:journeys
```

Add journeys under `packages/voting-portal/test/journeys/`, importing its `test`
fixture for a fresh browser context, clock and service mocks. The shared
`packages/ui-test-kit` validates GraphQL against the portal schema, checks OIDC
PKCE and owns ephemeral static-server ports. Register every service response;
unexpected requests fail teardown. Audit assertions decode downloaded ballots
with the vendored WASM in Node. CI uploads traces, screenshots and JUnit results
from `test-results/`. Known accessibility failures are marked only after the
journey and the exact known rule/target have been checked.

Voting matrices also exercise chooser eligibility, practice-election gating,
mandatory materials, review and cast errors, gold-session restoration, receipts,
and ballot lookup. Integrity stories encrypt real ballots before changing only
the recorded hash. Receipt tests independently decode the rendered QR and compare
the downloaded document bytes. Chromium can fetch downloads outside page routing,
so the receipt fixture also serves those same bytes from its owned loopback
server and removes them in teardown.

Infrastructure-recovery and receipt-polling tests first assert the exact existing
failure state, then mark only the missing recovery UI assertion as expected to
fail. A fixed defect becomes an unexpected pass. Those markers do not waive
unexpected requests or unhandled page exceptions.

Results portal journeys use the same boundaries and export a real SQLite fixture
with `sql.js`; the production browser reads it through its own WASM loader.
After building the shared UI packages, run:

```sh
yarn --cwd packages build:results-portal
yarn --cwd packages/results-portal test:types
yarn --cwd packages/results-portal test:journeys
```

Fixtures live in `packages/results-portal/tests/fixtures/`, browser cases in
`tests/journeys/`, and component interactions in `src/components/__stories__/`.
Use literal expected counts and scoped publications. A rejected publication or
artifact should settle without repeating authentication; include a valid control
and assert that a route change uses the new event's token.

The ballot verifier's `test:journeys` runs against its production build and the voting portal's production build. Run `yarn build:ui-core`, `yarn build:ui-essentials`, `yarn build:ballot-verifier`, and `yarn build:voting-portal` from `packages`, then `yarn --cwd ballot-verifier test:types` and `yarn --cwd ballot-verifier test:journeys`. Its Node fixture encrypts and signs real single- and multiple-contest ballots; the cross-portal case imports the exact voting-portal audit download. Invalid inputs first pass a valid control, then change only the signature, JSON, or supplied ballot ID. Confirmation stories and the production scan pin the existing candidate-list accessibility violation as expected failures, so fixing it requires removing the marker.

Admin production journeys use `yarn --cwd packages/admin-portal test:journeys` after building the shared UI packages and admin portal. `test:types` checks their fixtures; `typecheck:stories` checks admin stories. The fixture answers the known React-admin telemetry request locally and rejects every other unexpected service request. Tally and policy stories use strict data-provider and Apollo boundaries; form submission assertions check serialized policy values.

Admin journeys verify event creation/import, voter changes with confirmation and restricted permissions, session refresh/logout/tenant selection, and publication generation through voting closure. Story form assertions check each saved policy value. Shared story fixtures allow only the exact Vite/Vitest runner sockets; caught application WebSocket attempts and asset writes still fail teardown.

## Admin coverage before a refactor

The route smoke matrix follows the views declared in `admin-portal/src/App.tsx`.
Add a realistic fixture and a visible-content assertion when adding a route.
Area journeys provide the deeper workflows and assert exact mutation variables
or REST method, path and body. Keep pure data transformations in Jest and reserve
stories for interactions that are awkward to reach through a complete journey.

After building the shared packages and admin portal, collect all three layers
from the same source revision, then inspect their union:

```sh
yarn --cwd packages/admin-portal test --coverage --coverageReporters=json
yarn --cwd packages/admin-portal test:stories --coverage
STEP_UI_JOURNEY_COVERAGE=1 yarn --cwd packages/admin-portal test:journeys
node --experimental-strip-types packages/ui-test-kit/coverage/summary.mts journeys packages/admin-portal
yarn --cwd packages/admin-portal coverage:safety-net
yarn --cwd packages/admin-portal coverage:safety-net --prefix src/resources/ElectionEvent
```

The report counts a source line once when any layer executes it, using raw
Istanbul statement locations or LCOV line records. Its denominator contains
every line reported by any layer; it is not the sum or maximum of the layers'
summary percentages. Compare each file's uncovered lines and wire contracts
before changing its implementation. A missing layer is explicitly marked and
does not establish that the combined coverage is complete.

CI runs admin journeys in four shards with two workers each and publishes the
area and per-file tables in the safety-net job summary. The
`admin-portal-safety-net` artifact contains `coverage-union.json` and
`coverage-union.md`, including uncovered line ranges. Locally these files are
under `packages/admin-portal/test-results/safety-net/`. To combine downloaded
artifacts, pass repeated `--layer jest=<path>`, `--layer stories=<path>` and
`--layer journeys=<path>` options; directories are searched for raw line reports
and journey shards are united before the layer comparison.

CI step summaries list passes, expected failures (JUnit `fail`/`expected-failure` properties),
failures, skips and coverage as covered/total (percent): Istanbul for stories; for journeys, the
Istanbul statements, functions and branches of the bundled TypeScript `src/**`, each counted from the
innermost V8 block around its bundle code (unloaded chunks count 0), in `test-results/journey-coverage/`:
```sh
STEP_UI_JOURNEY_COVERAGE=1 yarn --cwd packages/voting-portal test:journeys
node --experimental-strip-types packages/ui-test-kit/coverage/summary.mts journeys packages/voting-portal
```
