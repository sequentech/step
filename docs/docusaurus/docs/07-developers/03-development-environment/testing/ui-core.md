---
id: ui-core
title: UI Core tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Test file paths in this guide are relative to [`packages/ui-core/tests/`](https://github.com/sequentech/step/blob/feat/meta-13302-ui-essentials-coverage/main/packages/ui-core/tests). Commands state their working directory.

Run from `packages/` after installing the workspace dependencies:

```bash
yarn --cwd ui-core test:types
yarn --cwd ui-core test:coverage
yarn --cwd ui-core test:browser
```

Use Node 22.22 or newer for the TypeScript browser runner. Install Google Chrome
before running browser tests. `UI_CORE_CHROME_PATH` can select another Chromium
executable; no browser download, remote election, identity service or paid test
service is required. Browser traffic is restricted to the temporary local fixture.

Source instrumentation runs before Babel creates re-export getters. The shared
`test-exclude` resolution supports the workspace's current `glob` API. Its
scoped `brace-expansion` override precedes the older workspace-wide override
because Yarn uses the first matching resolution; other consumers keep their
existing dependency. This keeps
compiler-generated code out of the branch denominator without excluding the
package entry point. Empty declaration/enum modules have no instrumentable
statements; their zero-count entries are not coverage failures.

The unit suite exercises language precedence and fallback, presentation ordering,
candidate categories, HTML sanitization, export cleanup, React refs and WASM
initialization. Regression inputs include prototype-like identifiers, encoded
cookies, malformed policies and zero candidate points. Fixtures contain only
synthetic election identities.

`wasm.test.ts` checks argument order, returned values and error propagation at
the adapter boundary with a stubbed engine. These tests do not verify cryptography.
The browser suite loads the real pinned `sequent-core` WASM binary, generates
and decodes its sample ballot, checks its receipt hash, confirms a changed issue
date changes the hash and rejects malformed encoded contests. It also exercises
cookies and sanitized HTML in Chromium. This is package integration coverage;
full voter journeys need authentication and service fixtures.

When changing the WASM dependency, rerun both suites. The test runner must fail
on an incompatible or missing binary rather than silently substitute a mock.

Some counted service paths have no test. In `src/services/votingPortalDateTime.ts`,
`tokenValue` keeps a defensive default that the parser cannot reach: it only
passes tokens matched by the supported-token pattern. Revisit it if the accepted
tokens change. `resolvePreset` runs only when a resolved formatter throws while
formatting, which the suite does not exercise. The remaining uncovered branch
alternatives in `i18n.ts`, `presentationOrder.ts`, `translate.ts` and
`translationScopes.ts` are reachable test gaps, not accepted exclusions.

## Assurance lint policy

Run `yarn lint` from this package. The existing frontend workflow runs it too.
Production source rejects explicit `any`, non-null assertions (`!`), TypeScript
suppression comments, unsafe `finally` blocks and returned Promise executor
values. Unit and browser tests retain their existing assertion and fixture style;
these additional production restrictions do not apply to tests or stories.

Presentation translations require own language and candidate-key properties.
Inherited scoped overrides and malformed language records fall back to a valid
own default; null-prototype dictionaries remain supported. The browser suite is
required by the frontend workflow and uses the same pinned headless shell
as the native fixture. Its counters remain separate from Jest source coverage.

Resource-failure controls exercise object-URL allocation and DOM insertion,
asserting the original error and exact cleanup even when those source lines
are also exercised by successful operations.
