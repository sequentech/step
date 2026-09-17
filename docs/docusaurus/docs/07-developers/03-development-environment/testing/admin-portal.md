---
id: admin-portal
title: Admin Portal tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Install locked dependencies from `packages/` with the pinned Node and Yarn, then:

```sh
yarn workspace admin-portal test --runInBand
yarn workspace admin-portal test --runInBand --coverage
```

Tests cover data preparation, dashboards, tally validation, secret-attribute
handling, GraphQL errors and permission selection. Permission fixtures distinguish
read/create/update/delete and trustee ceremony roles. Unknown names, including
JavaScript prototype property names, must return the explicit admin fallback.

Candidate/list tests use a literal policy table for all four selection modes,
missing presentation data and exact URL selection. Password tests replace only
the random-byte boundary to check alphabet mapping; random output is also checked
for length and allowed characters. These assertions do not measure randomness
quality.

The unit profile includes all executable source TypeScript/TSX, including
unimported components and generated runtime helpers. Only tests and declarations
are excluded, consistently across counters and exports. CI compares lines,
statements, functions and branches separately with the actual PR base. UI browser
interactions, Keycloak redirects, GraphQL services and the real cryptographic WASM
boundary remain separate integration scopes.
