---
id: results-portal
title: Results Portal tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

With the pinned Node and Yarn versions, install the locked workspace dependencies
from `packages/`, then run:

```sh
yarn workspace results-portal test --runInBand
yarn workspace results-portal test --runInBand --coverage
```

Tests exercise publication selection, result labels/order, rendered summaries,
GraphQL request and error contracts, public/authenticated artifact access, URL
resolution, area aggregation and CSS selection. Network fixtures use synthetic
`.invalid` URLs and inspect requests without contacting a service.

SQLite tests execute the bundled real SQLite asm engine in Node; only the
browser WASM-loader boundary is replaced. They check bound query parameters,
missing tables, exported database bytes and statement cleanup after failure.
SQLite may accept malformed bytes at construction and report the error only
when the first query reads a page; assertions observe that query boundary.

The Jest inventory includes all executable `src` TypeScript/TSX, including files
not imported by a test. Declaration and test files are excluded consistently
from JSON, LCOV, HTML and totals. Generated runtime helpers remain counted.
CI compares lines, statements, functions and branches separately against the
actual PR base using each revision's own tests and locked dependencies.

Node tests do not measure browser rendering/layout, Keycloak redirects or the
browser WASM download. Those boundaries need separate browser execution; a
passing unit run does not imply they were exercised.
