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

## Event and settings workflows

Production event and settings journeys live in `test/journeys/events/` and
`test/journeys/settings/`, with local fixture builders in each directory's
`data.ts`. After building the shared UI packages and admin portal as described in
[UI browser tests](./ui-browser-tests.md), run them from the repository root:

```sh
yarn --cwd packages/admin-portal test:journeys test/journeys/events test/journeys/settings --workers=2
yarn --cwd packages/admin-portal test:types
```

These journeys assert rendered outcomes and complete GraphQL variables or upload
bodies. Keep known-defect markers immediately before the failing assertion, after
verifying the setup and request. A captured promise rejection must match the
specific documented defect; unrelated requests and page exceptions still fail
the fixture. Password-policy boundaries belong in the Node Jest validator tests.

## Access workflows

The production journeys in `test/journeys/access/` cover voter and tenant-user
management, roles, approvals, reconciliation uploads, notifications and tenant
selection. Their local data builders serve the same strict GraphQL, OIDC and
object-storage boundaries as the other admin journeys. Prepare the production
bundle and browser as described in [UI browser tests](./ui-browser-tests.md),
then run from `packages/`:

```sh
yarn workspace admin-portal test:journeys 'test/journeys/access/[^/]+\.spec\.ts$' --workers=2 --repeat-each=3
```

Assert the full variables for each write, the permission role, and the visible
result. Upload checks include the exact presigned URL and bytes. After a write,
wait for its refreshed list data before opening another row action. Defect tests
must establish their setup before marking only the affected assertion as an
expected failure; unrelated service requests and browser errors still fail them.
