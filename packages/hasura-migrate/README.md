# hasura-migrate

Container image that applies Hasura SQL migrations + metadata from this step
commit (same release tag as other app images).

## What it does

Mirrors `gitops` workflow `hasura_apply_migration.yml`:

1. Wait for Hasura `/healthz`
2. Ensure Postgres source `backend-db` via metadata API (`pg_add_source`) if missing
3. Substitute `{{HARVEST_DOMAIN}}` in `metadata/actions.yaml` (default `harvest:8400`)
4. `hasura migrate apply --database-name backend-db --up all`
5. `hasura metadata apply`
6. Print `hasura migrate status`

## Runtime env

| Variable | Required | Default | Meaning |
|---|---|---|---|
| `HASURA_GRAPHQL_ENDPOINT` | yes | `http://hasura:8080` | In-cluster Hasura URL |
| `HASURA_GRAPHQL_ADMIN_SECRET` | yes | — | Admin secret |
| `HARVEST_DOMAIN` | no | `harvest:8400` | Action handler host:port |
| `HASURA_DATABASE_NAME` | no | `backend-db` | Source name |
| `ENSURE_PG_SOURCE` | no | `true` | First-deploy DB connect |
| `PG_DATABASE_URL_ENV` | no | `PG_DATABASE_URL` | Hasura env var holding DB URL |
| `WAIT_TIMEOUT_SECONDS` | no | `300` | Health wait |

## Build

Built by release matrix as service `hasura-migrate` from
`packages/Dockerfile.hasura-migrate` (context: repo root).
