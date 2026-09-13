<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# UI Core tests

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

The existing frontend CI job calls `yarn test`, which now runs the type check and
strict coverage gate. For a focused development loop, run `yarn jest <test-file>
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
full voter journeys remain separate work in Meta #13298.

When changing the WASM dependency, rerun both suites. The test runner must fail
on an incompatible or missing binary rather than silently substitute a mock.
