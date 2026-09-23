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
| `PG_DATABASE_URL` | Postgres URL for source `backend-db` |
| `HASURA_GRAPHQL_ADMIN_SECRET` | Admin secret |
| `HARVEST_DOMAIN` | Host:port for action handlers (`{{HARVEST_DOMAIN}}` in metadata), e.g. `harvest:8400` |

Do **not** replace the image entrypoint with bare `graphql-engine serve` or auto-apply is skipped. To keep log tee for JWK probes, keep entrypoint and only override **args**/CMD.
