---
id: package-test-inventory
title: Package test inventory
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

A directory under `packages/` is not necessarily a member of the root Rust or
JavaScript workspace. Check `packages/Cargo.toml`, `packages/package.json`, nested
Cargo workspaces and Maven reactors before interpreting a package test run.

| Area | Test entry point | Separate boundary |
| --- | --- | --- |
| Bulletin board and trustee protocol | [Wbraid board storage](./b4.md), [native protocol](./braid.md) | Trustee transport, custody and persistent storage |
| Native board audit storage | [Electoral log](./electoral-log.md) | PostgreSQL recovery, signatures and receipt contracts |
| Frontends | [Results Portal](./results-portal.md), [Ballot Verifier](./ballot-verifier.md), [Admin Portal](./admin-portal.md) | Browser flows and live services |
| Java utilities and extensions | [ECIES](./ecies-encryption.md), [Keycloak](./keycloak-extensions.md) | Certificate chains and deployed authentication |
| Load-test helpers | [E2E helpers](./e2e-helpers.md) | Remote browser/load scenarios |
| Runtime adapters and rendering | [Orare](./orare.md) | Deployed HTTP and S3 transfer |
| Plugin component | [Miru](./miru.md) | Host JWT and transaction implementations |
| Backend database and Hasura | [Backend database contracts](#backend-database-contracts) | Populated deployments and live services |

The supplied `wbraid` directory is a separate Cargo workspace with existing
protocol, model/property, persistence and cross-implementation tests. Run from
`packages/wbraid`:

```sh
CARGO_TARGET_DIR="$PWD/../rust-local-target" cargo test --locked --release --features sqlite,postgres
```

Read that workspace's feature and integration-test prerequisites before enabling
additional backends. Ignored external-service/JVM cases and the live b4v6/LocalStack
fixtures remain separate obligations; enabling a feature alone does not prove
its external boundary was exercised.

Use the coverage profiles only for their named configurations. A passing package
suite without an instrumented source inventory has no implied percentage.
Unimported frontend source remains counted; test-only setup/declarations and
separate browser/WASM/runtime profiles must be accounted for explicitly.

## Backend database contracts

The `Backend database contracts` workflow uses fresh PostgreSQL 18 databases.
The migration check applies every Hasura backend-db migration, rolls back all
but the squashed baseline and requires its schema immediately after rollback.
It then reapplies them and compares with the fully migrated schema. Dump nonces
and physical column order are ignored; column types, defaults, nullability and
constraints remain part of the comparison. The
Hasura check starts the devcontainer's Hasura image, which applies the
migrations and metadata, and requires consistent metadata. From the repository
root:

```sh
docker run --rm --user postgres --volume "$PWD:/step:ro" --workdir /step \
  postgres:18-bookworm scripts/postgres/check_migrations.sh
scripts/postgres/check_hasura_metadata.sh
docker build --tag voting-flow-harness scripts/voting_flow
docker run --rm --volume "$PWD:/step:ro" --workdir /step voting-flow-harness \
  python3 scripts/test_cast_vote_scalability.py
yarn --cwd hasura install --frozen-lockfile
node --test hasura/tests/document-access.test.cjs
```

Run `scripts/test_scheduled_voting_channels.py`,
`scripts/test_ballot_publication_lifecycle.py` and
`python3 -m unittest discover -s packages/voting-load` in the same image. Inside
devenv, run the Python commands and `check_migrations.sh` directly;
`python3 scripts/test_cast_vote_scalability.py --rust-tests` also runs the
ignored Windmill `services::insert_cast_vote::tests` against its disposable
database.

The Tests workflow's Windmill job runs those ignored tests against the same
fixture in a Docker container. From `packages/`:

```sh
export CAST_VOTE_TEST_DATABASE_URL=$(../scripts/voting_flow/castvote_fixture.sh)
cargo test --locked -p windmill --lib services::insert_cast_vote::tests -- --ignored
docker rm --force --volumes step-castvote-fixture
```
