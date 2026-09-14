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

## Coverage and remaining work

`test:coverage` enforces 95% lines, statements, functions and branches. The
baseline command deliberately reports the current unfinished result without
enforcing that threshold. The package has not reached 95%; do not treat a passing
baseline or ordinary unit job as completion of that target.

Coverage includes all runtime TypeScript under `src`, even modules no test
imports. Only declaration files, tests and Jest support are excluded. Babel
instruments executable source before transformation, so erased TypeScript
declarations are not counted as unexecuted application statements. HTML, JSON
and LCOV reports are written to `coverage/`.

The initial source baseline had 139 passing tests and 27.85% line coverage.
This increment has 164 passing unit tests and 34.71% line coverage, plus the two
separate browser tests. Authentication, GraphQL failure handling, ballot
encryption/submission and complete voter journeys still need service fixtures.
Browser execution is not included in the Jest coverage totals.

`test:types` checks the source and test dependency graph. At this increment it
reports the same pre-existing diagnostics as the unchanged baseline, including
Apollo version/type incompatibilities, translation keys and shared component
types. It is not a passing gate yet. The existing CI unit command is unchanged;
run both the coverage and type commands when reviewing follow-up work.

If the browser fixture cannot render, the runner includes its browser errors
alongside the locator failure. First check that the local WASM archive is
installed and that the Chromium executable is available. Keep the fixture local;
do not substitute a deployed election to make a package test pass.

## Follow-up contracts

The browser fixture compiles Question with the production TypeScript target before bundling. This reproduced a category toggle that appeared expanded locally while its parent still considered every category collapsed: ES5 computed-key assignment lost __proto__. The handler now writes into a null-prototype object. Four additional component cases exercise individual categories and the aggregate control. 168 unit tests and two browser tests pass; the same 20 pre-existing TypeScript diagnostics remain. Source coverage is 849/2440 lines, 172/555 functions and 479/1714 branches. Full identity/submission workflows remain separate service-fixture obligations.

CI compares actual PR revisions with the same Babel inventory and rejects any
decrease in lines, statements, functions or branches. At `a657eef`, source
coverage is 849/2,440 lines, 870/2,503 statements, 172/555 functions and 479/1,714
branches, with 168 passing unit tests and two browser tests. The production
ES5 browser fixture reproduces the prototype-like category ID regression before
the fix. Twenty existing TypeScript diagnostics match the parent; none were added.
