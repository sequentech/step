---
id: ballot-verifier
title: Ballot Verifier tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Install the locked workspace dependencies from `packages/` with the pinned Node
and Yarn versions, then run:

```sh
yarn workspace ballot-verifier test --runInBand
yarn workspace ballot-verifier test --runInBand --coverage
yarn workspace ballot-verifier test:types
```

The Jest profile runs in jsdom with `?lang=en`. Besides service, store and
provider contracts, it renders the import and confirmation screens and the event
routes (`/tenant/:tenantId/event/:eventId/{login,start,confirmation}`), with App
mounted in the provider tree of `src/index.tsx`. Assertions use roles, labels and
visible text. App tests live in `src/App.routing.test.tsx`; `src/App.test.tsx` is
an obsolete scaffold outside the profile. With authentication disabled, root and
direct login links reach the appropriate event's import step, and verification
continues to confirmation without Keycloak or private GraphQL requests.

Test doubles live in `src/__mocks__`:

- `sequentCore.ts` replaces the sequent-core WASM bindings in every test. Ballot
  operations throw until a test records an answer, so unexpected calls fail.
- `auditableBallots.ts` builds the Playwright journeys' single- and
  multiple-contest ballots and records sequent-core's answers for them, following
  its Rust contract: each format rejects the other, and a changed or incomplete
  signature fails verification.
- `keycloak.ts` is a fake keycloak-js client. Install it with
  `jest.mock("keycloak-js", () => jest.requireActual("<path>/__mocks__/keycloak"))`
  and adjust `keycloakSession` before rendering.

`fetch` rejects every request unless a test stubs it. The App test answers the
settings, S3 and Hasura requests the journeys' mocks serve, and fails on any other.

To add a test, record the sequent-core answers it needs with `recordSequentCore`
or `sequentCore.<binding>.mockImplementation`, and take expected values from the
fixtures or the documentation rather than from the code under test. Start
rejection cases from a valid import. Known defects are pinned with `it.failing`
and a one-line reason; fixing one makes its test fail until the marker is removed.

Mocked bindings check how the verifier uses sequent-core, not the cryptography.
Real encryption, hashing and signature checks run in the Storybook stories and
Playwright journeys; see [UI browser tests](./ui-browser-tests.md).

All executable TypeScript/TSX source remains in the coverage inventory, including
generated runtime helpers. Tests, declarations, test doubles, Storybook fixtures
and test setup are excluded consistently from counters and exports. CI compares
lines, statements, functions and branches separately against the actual PR base
using the same unit profile and each revision's own tests.


With authentication disabled, event routes may fetch their public
`election_event_config.json` to apply the event language policy and scoped
translations. This optional request never authenticates, queries private Hasura
data or blocks importing a local ballot when metadata is unavailable. The App
routing tests use actual configuration/translation services and verify these
boundaries. Direct login redirects replace their browser-history entry so Back
can leave the verifier instead of entering a redirect loop.
