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

### Upgrading databases that recorded version 1794700001600

Hasura records the numeric version, not the migration name. Canonical OVCS
version `1794700001600` is `cast_vote_seal_key` and remains unchanged. Earlier
branches used the same number for the operational demo installation or
protected configuration-package intake. These now use `1794700001610` and
`1794700001650`, respectively.

Back up first, then rehearse on an isolated restore with writers and other
migrators stopped. Run normal Hasura migrations even if later versions are
already recorded. A fresh database applies all three versions in order. On
an existing database, 1610 verifies and preserves complete cast and demo
schemas and installs only the wholly absent feature. It briefly locks the
tenant and election tables and never calls the demo registration routine.
Version 1650 verifies and preserves exact existing intake columns, constraints,
index and bound credentials, or adds the schema without a backfill. None
creates a tenant, election, key or demo installation. An installation already
on 1650 with canonical cast sealing applies the pending 1610 normally.

If a provisional portable installation recorded 1600 **only** for
`configuration_package_unwrap`, 1610 refuses because neither cast sealing
nor the operational demo schema is present. On the isolated restore, an
authorized database operator should verify the exact historical staging schema
and confirm all database-local canonical cast objects are absent. Then run
the unchanged `1794700001600_cast_vote_seal_key/up.sql` once in a transaction
and rerun normal migrations so 1650 adopts the existing staging schema.
If a schema is partial or has unexpected definitions or grants, stop and
review it. Do not replay SQL over partial objects, edit Hasura's migration
records, use `--skip-execution`, or reset existing data.

Rollback of 1610 removes only empty demo schema: it never removes cast keys
or requirements and refuses for a populated demo. Rollback of 1650 refuses
while any staging row retains bound intake fields.

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
