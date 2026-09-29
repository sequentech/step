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
```

The unit profile exercises service, store and provider contracts: ballot-style
ordering and JSON decoding, election-scoped replacement, selector fallbacks,
confirmation contests, language selection and WASM readiness gating. Rejection
cases include valid controls; expected field mappings and ordering are literal.
The readiness test replaces the external WASM loading boundary and renders the
real gate using React's server renderer.

All executable TypeScript/TSX source remains in the coverage inventory, including
unimported screens and generated runtime helpers. Tests, declarations, Storybook
fixtures and test setup are excluded consistently from counters and exports.
CI compares lines, statements, functions and branches separately against the
actual PR base using the same unit profile and each revision's own tests.

This profile selects service, store and provider tests. The root `App.test.tsx`
is an obsolete Create React App scaffold; browser routing, layout and loading the
real cryptographic WASM module need a separate browser harness. These boundaries
are not validated by a passing Node unit suite.
