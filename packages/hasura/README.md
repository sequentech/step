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
