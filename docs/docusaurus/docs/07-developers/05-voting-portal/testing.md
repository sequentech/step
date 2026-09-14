---
id: testing
title: Voting Portal tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Test file paths in this guide are relative to [`packages/voting-portal/tests/`](https://github.com/sequentech/step/blob/feat/meta-13302-ui-essentials-coverage/main/packages/voting-portal/tests). Commands state their working directory.

Install the locked workspace dependencies from `packages/`, then run:

```bash
yarn --cwd voting-portal test --runInBand
yarn --cwd voting-portal test:coverage:baseline
yarn --cwd voting-portal test:browser
```

The browser runner needs Node 22.22 or newer and Google Chrome. Set
`VOTING_PORTAL_TEST_CHROME_PATH` to use another Chromium executable. It serves a
synthetic ballot on an ephemeral loopback port and blocks requests to other
origins. No election server, identity provider, production credential or paid
test service is required.

## What the tests establish

The unit suite covers candidate display and selection policies, invalid ballot
markers, session cleanup, and existing keyboard and accessibility behavior.
Category regressions exercise the actual Question, AnswersList and shared
CandidatesList components. They use names such as `__proto__` and `constructor`
to catch inherited-object lookups and accidental mutation during rendering.
Candidate inputs and vote interpretation are stubbed in those unit tests.

The two Chromium tests load the real components, Redux store and pinned local
WASM engine. They check repeated category expansion, keyboard activation,
candidate selection, and preservation of that selection when its category is
hidden. No candidate or selection implementation is mocked. The fixture uses
a small UI Essentials entry file to bundle only the real components it needs.
This is a browser integration test, not an authenticated end-to-end election.

Coverage includes all runtime TypeScript under `src`, even modules no test
imports. Only declaration files, tests and Jest support are excluded. Babel
instruments executable source before transformation, so erased TypeScript
declarations are not counted as unexecuted application statements. HTML, JSON
and LCOV reports are written to `coverage/`.

If the browser fixture cannot render, the runner includes its browser errors
alongside the locator failure. First check that the local WASM archive is
installed and that the Chromium executable is available. Keep the fixture local;
do not substitute a deployed election to make a package test pass.

## Category state and source coverage

Compile browser fixtures with the production TypeScript target before bundling.
Prototype-like category IDs must preserve both individual and aggregate expanded
state; null-prototype dictionaries avoid inherited lookups and ES5 computed-key
assignment surprises. Exercise individual categories and the aggregate control.

CI compares actual PR revisions with the same Babel inventory and rejects any
decrease in lines, statements, functions or branches. Browser execution has
separate accounting. Authentication, GraphQL failure handling and ballot
submission need explicit service fixtures; do not substitute deployed elections.
Run `test:types` as well as unit tests and inspect any dependency diagnostics.
