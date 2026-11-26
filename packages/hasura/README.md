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

Hasura records the number, not the migration directory name. On OVCS,
`1794700001600_cast_vote_seal_key` is immutable. An older HSM branch used
that same number for `operational_signing_demo_installation`. The latter
now has its own number, `1794700001610`, before self-setup at 1700.

Use the ordinary Hasura migration command, including when later versions
are already recorded. Back up and rehearse on an isolated restore first.
Stop writers/other migrators during the upgrade; this migration briefly locks
the tenant and election tables. Version 1610 runs in one transaction:

| Actual database before 1610 | Result |
| --- | --- |
| Fresh release or canonical cast sealing only | Keep cast policy and any existing keys; install **inert** demo DDL. |
| Original demo only | Keep demo state and grants; install the exact canonical cast-sealing DDL. No event is enrolled or sealed by default. |
| Both historical schemas | Verify and keep both without replacing any object. |
| Neither, partial, or unexpected security definitions | Stop, with no partial upgrade. Inspect the catalog and restore/repair the discrepancy with the database owner; never guess the applied branch from the number. |

No tenant, event, key, device or demonstration installation is created by
1610. In particular, it never calls the demo registration routine. It checks
the expected table shape, constraints, grants, functions and enabled policy
triggers before accepting an installation. Unexpected direct grants are not
silently revoked. Never edit Hasura's version records, use `--skip-execution`,
reset existing data, or replay canonical 1600 over partially present objects.
Rollback of 1610 can only undo **empty** demo DDL; an installed demo refuses,
and cast-sealing flags, keys and privileges remain owned by canonical 1600.

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
