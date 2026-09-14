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

The coverage command requires 95% lines, statements, functions and branches.
It includes every source module, including modules no test imports, and writes
HTML, JSON and LCOV to `coverage/`. Only test files and declaration files are
excluded. `test:coverage:baseline` reports an unfinished result without enforcing
the target; use the strict command for review evidence.

The ordinary frontend job runs unit tests and type checking. The separate
coverage job measures both actual PR revisions and rejects any decrease in
lines, statements, functions or branches; 95% remains the local improvement target. For a focused development loop, run `yarn jest <test-file>
--runInBand`; the real browser suite is a separate command.

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
full voter journeys remain separate work in [Meta #13298](https://github.com/sequentech/meta/issues/13298).

When changing the WASM dependency, rerun both suites. The test runner must fail
on an incompatible or missing binary rather than silently substitute a mock.

## Assurance lint policy

Run `yarn lint` from this package. The existing frontend workflow runs it too.
Production source rejects explicit `any`, non-null assertions (`!`), TypeScript
suppression comments, unsafe `finally` blocks and returned Promise executor
values. Unit and browser tests retain their existing assertion and fixture style;
these additional production restrictions do not apply to tests or stories.

## Follow-up contracts

Two additional resource-failure controls exercise object-URL allocation and DOM insertion failures, asserting the original error and exact cleanup. The 227-test suite, type/lint checks and four real Chromium/WASM tests pass. At `496e36a`, source metrics are 744/750 lines, 776/782 statements, 177/178 functions and 350/363 branches. Each metric improves against the actual PR base. These paths add failure evidence even where line counters were already covered.

Presentation translations require own language and candidate-key properties.
Inherited scoped overrides and malformed language records fall back to a valid
own default; null-prototype dictionaries remain supported. The browser suite is
now required by the frontend workflow and uses the same pinned headless shell
as the native fixture. Its counters remain separate from Jest source coverage.

## Remaining measured paths

The uncovered function is the unused internal `resolvePreset` callback in
`services/votingPortalDateTime.ts`; no public path calls it. It stays counted,
rather than exporting or invoking dead code only to raise coverage.

| Source under `src/services/` | Missing branch alternatives | Remaining behavior |
| --- | ---: | --- |
| `i18n.ts` | 7 | Absent configuration/resources, language-policy fallbacks and removal of an already empty override layer. |
| `presentationOrder.ts` | 1 | Null right-hand label in alphabetical ordering. |
| `translate.ts` | 1 | An own `i18n` property explicitly set to `undefined`. |
| `translationScopes.ts` | 3 | Retaining an existing key, omitting legacy entries and selecting the global scope directly. |
| `votingPortalDateTime.ts` | 1 | Defensive token-switch default after the parser has validated its token set. |

The first four rows remain reachable test opportunities. The date parser's
fallback must be revisited if its accepted token set changes. Native WASM crypto
internals remain separate from these Istanbul counters. Browser receipt tests
independently encode the public raw-ballot envelope and verify SHA-512 using
Node crypto, rather than using another WASM alias as the only hash oracle.
