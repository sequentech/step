<!--
SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# hasura (Sequent image)

Thin packaging of upstream `hasura/graphql-engine:<ver>.cli-migrations-v3` with
this repo's `hasura/migrations` and `hasura/metadata` baked in.

CI build context is `hasura/` (not repo root `.`); Dockerfile `COPY`s
`migrations` and `metadata` from that directory. Dockerfile path remains
`packages/Dockerfile.hasura`.

## Runtime

Upstream `docker-entrypoint.sh` on start:

1. Temporary engine on :9691
2. `metadata apply` from `/hasura-metadata` (includes `backend-db` → `from_env: PG_DATABASE_URL`)
3. `migrate apply --all-databases` from `/hasura-migrations`
4. `metadata reload`, then `exec` normal serve

### Configuration-package credential intake migration

`backend-db` version `1794700001600` belongs to
`cast_vote_seal_key`. The protected configuration-package intake migration is
now `1794700001650_configuration_package_credential_intake`. On a fresh
database Hasura applies both normally; neither creates a tenant or election.

An earlier portable development branch used version `1794700001600` for
`configuration_package_unwrap`. A separate HSM demo lineage used the same
number for `operational_signing_demo_installation`. Hasura records only the
number. Do not infer which SQL ran from `hasura migrate status`, edit an
applied version, or use `--skip-execution` to make the history look clean.
Version 1650 refuses to start with missing or incomplete canonical cast-vote
objects or a partial credential-intake schema.

- For a pristine install, use new databases and object-storage volumes.
  Leave any old database, vault state, and staged objects intact.
- To upgrade a prior portable installation with state you must keep, take
  a backup, stop writers and work on an isolated restore first. Compare its
  actual schema, constraints, triggers and privileges with the exact
  migrations from both releases. If the portable staging schema is correct
  and the entire canonical cast-vote migration is absent, an authorized
  database operator can run that exact canonical `1600` **up.sql** in one
  transaction on the restore, without modifying any migration records.
  Then run the usual Hasura migration: 1650 verifies and adopts the exact
  pre-existing staging schema without changing any staged record or
  credential. Do not run that repair SQL over partially present cast-vote
  objects. The separate HSM demo history requires its own reviewed upgrade;
  this release does not install or remap those HSM migrations.

## Required pod env

| Var | Purpose |
|-----|---------|
| `HASURA_GRAPHQL_METADATA_DATABASE_URL` | Postgres URL for Hasura's **metadata catalogue** (`hdb_catalog`). Required for the engine (and the cli-migrations temp server) to start. Often the same DB as the app source in our deploys. |
| `PG_DATABASE_URL` | Postgres URL for tracked source `backend-db` (`from_env` in metadata) |
| `HASURA_GRAPHQL_ADMIN_SECRET` | Admin secret |
| `HARVEST_DOMAIN` | Host:port for action handlers (`{{HARVEST_DOMAIN}}` in metadata), e.g. `harvest:8400` |

`PG_DATABASE_URL` alone is **not** enough: it wires the GraphQL data source; it does not replace the metadata DB URL.

## Security

Image runs as non-root user **`hasura` (UID/GID 1001)**. Prefer matching `securityContext` / `runAsUser: 1001` in gitops if you set an explicit user.

Do **not** replace the image entrypoint with bare `graphql-engine serve` or auto-apply is skipped. To keep log tee for JWK probes, keep entrypoint and only override **args**/CMD.
