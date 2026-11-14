<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# hasura threat model

`hasura` is the Hasura GraphQL Engine image built by `packages/Dockerfile.hasura`, together with the metadata and migrations in the repository's top-level `hasura/` directory. It runs as a server container in front of the backend PostgreSQL database. admin-portal, voting-portal, results-portal, ballot-verifier, the IVR integration and the backend services all use it, and every GraphQL action it exposes is forwarded to harvest. Its role permissions are the main authorization layer for election data that clients reach directly, so a gap here can break tenant isolation, ballot secrecy or election integrity. This package directory holds only a README, so the paths below are relative to the repository root. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **Tenant isolation of `sequent_backend` rows**: `hasura/metadata/databases/backend-db/tables/tables.yaml` tracks 43 tables shared by all tenants, with 470 grants across 78 roles. Confidentiality and integrity.
- **Election public key and ballot styles**: `election_event.public_key`, `ballot_style.ballot_eml` and `ballot_signature`. Voters encrypt to them. Integrity.
- **Cast votes**: `cast_vote.content`, `voter_id_string`, `cast_ballot_signature`, and `annotations`, which holds the voter's IP, country and voting channel (`packages/windmill/src/postgres/cast_vote.rs` `cast_vote_annotations`). Integrity, and confidentiality of the voter-ballot link.
- **Trustee and ceremony data**: `trustee.public_key`, `keys_ceremony`, `tally_session*`. Integrity of the threshold trust model.
- **Results and publication policy**: `results_*`, `tally_results_publication`, `election_event.presentation.results_website`. Integrity, and confidentiality before publication.
- **Scheduled events**: `scheduled_event` rows, which windmill turns into voting-period, tally, enrollment and lockdown changes. Integrity.
- **Voter and applicant personal data**: `applications.applicant_data`, `phone_blacklist.phone_e164`, `tasks_execution.logs`. Confidentiality.
- **Document metadata and access annotations**: `document.annotations`, which governs access to exports and reports stored in MinIO. Confidentiality and integrity.
- **Stored secrets and integration credentials**: `sequent_backend.secret`, whose values windmill encrypts with a master key (`packages/windmill/src/services/vault/vault.rs` `save_secret_and_return`), and the credentials that integrations use. Confidentiality.
- **Secret-bearing action inputs**: private keys (`check_private_key`, `restore_private_key`), passwords (`exportTrustees`, `encrypt_report`, `upload_signature`) and OTPs (`set_voter_authentication`) in `hasura/metadata/actions.graphql`. Confidentiality.
- **Admin secret and `service-account` credential**: either one bypasses all row scoping. Confidentiality.
- **JWT trust anchor**: the JWKS document named by `HASURA_GRAPHQL_JWT_SECRET`. Integrity.
- **Permission labels**: `permission_label` columns, which separate admins inside one tenant. Integrity.
- **GraphQL API during voting**: availability.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| `/v1/graphql` over HTTP and WebSocket, tables in `hasura/metadata/databases/backend-db/tables/*.yaml` | Browsers and services holding a Keycloak JWT | authenticated voter / authenticated admin / internal service | The JWT is verified against the JWKS in `HASURA_GRAPHQL_JWT_SECRET`. The role is chosen from `x-hasura-allowed-roles`. Each role has its own row filter, check and column list |
| Requests without a token | Anyone | untrusted | `HASURA_GRAPHQL_UNAUTHORIZED_ROLE=unauthorized` in the compose files. That role can select 4 columns of `sequent_backend_tenant` and has no actions |
| 120 actions in `hasura/metadata/actions.yaml` (types in `actions.graphql`) | Roles listed per action. The voter role `user` has 8 of them, including `insert_cast_vote` | authenticated voter / authenticated admin | The per-action role list. All are synchronous calls to `http://{{HARVEST_DOMAIN}}/...`. A Kriti `request_transform` builds each request body from the action input. harvest applies its own authorization checks (see the [harvest threat model](../harvest/THREAT_MODEL.md)) |
| 7 IVR REST endpoints under `/api/rest/ivr/` in `hasura/metadata/rest_endpoints.yaml`, backed by `allowed-queries` in `query_collections.yaml`; `IvrInsertCastVote` (POST) calls the `insert_cast_vote` action | IVR integration holding event-realm tokens | internal service / authenticated voter | The same role permissions and action checks as GraphQL for the caller's token |
| `/v1/metadata`, `/v2/query`, console | Holders of the admin secret | operator / internal service | `HASURA_GRAPHQL_ADMIN_SECRET`. The console is controlled by `HASURA_GRAPHQL_ENABLE_CONSOLE` |
| Container start (cli-migrations entrypoint) | Container runtime, CI | operator | Applies the metadata and migrations baked into the image by `packages/Dockerfile.hasura` |
| DB triggers in `hasura/migrations/backend-db` | Any writer to `cast_vote` or `election_event` (Hasura, harvest, windmill) | internal service | `check_revote_limit`, `guard_results_website_policy_update` |

No event triggers, cron triggers or remote schemas are configured (`hasura/metadata/cron_triggers.yaml`, `remote_schemas.yaml`, and no `event_triggers` in the table files). The outbound calls are the action webhooks to harvest and the JWKS fetch. Time-based state changes run in the windmill beat from `scheduled_event` rows, not in Hasura cron.

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| hasura-T1 | Spoofing | Forged or widened session claims (tenant, event, area, authorized elections, roles, labels) | Hasura runs in JWT mode, so clients cannot set `x-hasura-*` values. The token is verified against the JWKS named by `HASURA_GRAPHQL_JWT_SECRET` | Not verified |
| hasura-T2 | Information disclosure | Cross-tenant reads through GraphQL | Role grants filter on `tenant_id: {_eq: X-Hasura-Tenant-Id}` in `hasura/metadata/databases/backend-db/tables/*.yaml`. `service-account` grants are unscoped by design | Partial |
| hasura-T3 | Tampering | Cross-tenant or cross-event writes through GraphQL | Role grants scope writes by `tenant_id`. Some tables have composite foreign keys on `tenant_id` and `election_event_id` (`hasura/migrations/backend-db`) | Partial |
| hasura-T4 | Information disclosure | A voter reads data outside their own event, area or authorized elections | Role `user` grants filter on the voter's session variables (`X-Hasura-Tenant-Id`, `X-Hasura-Election-Event-Id`, `X-Hasura-Area-Id`, `X-Hasura-Authorized-Election-Ids`, `X-Hasura-User-Id`) | Partial |
| hasura-T5 | Tampering | A voter reads or alters another voter's ballot | On `sequent_backend_cast_vote.yaml`, insert, update and delete are granted to `service-account` only. The `user` select requires `voter_id_string = X-Hasura-User-Id` plus the event, area and election filters. Ballots enter through the `insert_cast_vote` action | Mitigated |
| hasura-T6 | Tampering | Double voting or exceeding the revote limit through concurrent inserts | `check_revote_limit` in `hasura/migrations/backend-db/1788765000000_serialize_cast_vote_area_checks/up.sql`: a `pg_advisory_xact_lock` per voter and election, a cross-area exclusivity check, and the `num_allowed_revotes` limit | Mitigated |
| hasura-T7 | Elevation of privilege | An admin, trustee or restricted admin acts with a broader role than intended | Allowed roles come from Keycloak realm roles in the token, and each role has its own grants per table | Partial |
| hasura-T8 | Elevation of privilege | Bypass of permission-label segregation between admins of one tenant | Grants on election-scoped tables use `X-Hasura-Permission-Labels` | Partial |
| hasura-T9 | Tampering | An admin alters election-critical data (keys, ballot styles, tally, results, schedules) without authorization | Role grants limit writes per table, and some election-critical tables are writable only by `service-account`. End-to-end verifiability is provided outside this package | Partial |
| hasura-T10 | Tampering | The results-website access policy is changed without publish rights | Trigger `sequent_backend.guard_results_website_policy_update` (`hasura/migrations/backend-db/1783000001000_guard_results_website_policy/up.sql`) requires role `publish-results-write` or `service-account` to change `presentation.results_website` | Partial |
| hasura-T11 | Information disclosure | Stored secrets are read through GraphQL | `sequent_backend_secret.yaml` is tracked with no role permissions, so only the admin secret reaches it. Values are encrypted by windmill `save_secret_and_return` before they are stored | Mitigated |
| hasura-T12 | Elevation of privilege | Disclosure of the admin secret, which gives `/v1/metadata`, `run_sql` and unscoped data access | The secret is set only through env (`HASURA_GRAPHQL_ADMIN_SECRET`), not in metadata. The DB URL comes from `from_env: PG_DATABASE_URL` in `hasura/metadata/databases/databases.yaml` | Partial |
| hasura-T13 | Spoofing | Identity forged or replayed on the Hasura to harvest action hop | Handlers are plain `http://{{HARVEST_DOMAIN}}`. How harvest authenticates the request, and whether only Hasura can reach it, are covered by harvest-T1 in the [harvest threat model](../harvest/THREAT_MODEL.md) | Not verified |
| hasura-T14 | Elevation of privilege | An action is invoked by a role below the privilege it needs | Each action has a `permissions` role list, and no action is granted to the `unauthorized` role. harvest applies its own authorization checks (see the [harvest threat model](../harvest/THREAT_MODEL.md)) | Partial |
| hasura-T15 | Information disclosure | Unauthenticated callers read tenant data | The `unauthorized` role has one select grant (`sequent_backend_tenant.yaml`: `id`, `slug`, `is_active`, `annotations`) and no actions | Accepted (the login page needs the tenant slug, status and theme before authentication; every tenant row is listable) |
| hasura-T16 | Repudiation | Admin data changes cannot be attributed | harvest and windmill write electoral-log entries for the actions they run | Partial |
| hasura-T17 | Denial of service | Expensive queries, aggregates or subscriptions overload Postgres during voting | `hasura/metadata/api_limits.yaml` is `{}` and `allow_list.yaml` is `[]`, so depth, node, rate and time limits are left to the deployment ingress | Not verified |
| hasura-T18 | Information disclosure | Console, dev-mode error details or query logs expose internals or secret-bearing action inputs | Controlled by `HASURA_GRAPHQL_ENABLE_CONSOLE`, `HASURA_GRAPHQL_DEV_MODE`, `HASURA_GRAPHQL_ENABLED_LOG_TYPES` and `HASURA_GRAPHQL_CORS_DOMAIN`, which the image does not set. The development and remote-demo env files under `.devcontainer/` enable the console, dev mode and `query-log` | Not verified |
| hasura-T19 | Information disclosure | Schema discovery through introspection | Introspection is enabled for all roles (`hasura/metadata/graphql_schema_introspection.yaml`) | Accepted (the metadata is public in this repository, so the schema is not secret) |
| hasura-T20 | Elevation of privilege | DB functions or migrations run with elevated rights, build SQL dynamically or seed credentials | `hasura/migrations/backend-db/*/up.sql` contain no `SECURITY DEFINER`, dynamic `EXECUTE`, `GRANT`, row-level-security changes or seed data. Trigger functions are invoker-rights plpgsql | Mitigated |
| hasura-T21 | Tampering | A compromised or outdated engine image | `packages/Dockerfile.hasura` pins the engine version, runs as non-root `USER hasura`, and bakes in the metadata and migrations | Partial |
| hasura-T22 | Tampering | Permission changes merged without review | Code review. `.github/workflows/graphql_schema.yml` starts the engine with the committed metadata and checks generated schemas for drift | Partial |
| hasura-T23 | Information disclosure | Admins link voter identities to encrypted ballots and to the voter's IP and country | Admin read roles on `sequent_backend_cast_vote.yaml` are tenant-filtered. Ballots are encrypted to the threshold election key and mixed by braid before decryption, outside this package | Not verified |
| hasura-T24 | Information disclosure | Integration credentials stored in the database are disclosed | `sequent_backend.secret` with windmill vault encryption is available for secrets | Not verified |
| hasura-T25 | Spoofing | Client network metadata (IP, country) stored with votes is spoofed | Deployment: the edge proxy sets the client IP and country headers | Not verified |
| hasura-T26 | Tampering | Scheduled voting-period, tally or lockdown changes are created or altered without authorization | The `createScheduledEvent` and `manage_election_dates` actions create scheduled events | Partial |
| hasura-T27 | Elevation of privilege | Roles kept in metadata for testing or platform administration become live if any realm issues them | `test-all` (`hasura/metadata/inherited_roles.yaml`, inherits `admin-user`) and `super-admin-user` (`sequent_backend_tally_session_resolution.yaml`) are not issued by any realm template in the repository | Not verified |
| hasura-T28 | Denial of service | Malformed `call_plugin_route` input crashes the handler: non-object `data` panics harvest `POST /plugin` (`packages/harvest/src/routes/plugins.rs` `plugin_routes`) | No control on release/10.0; the fix is in sequentech/step#3522 | Open on release/10.0 (fix in sequentech/step#3522) |
| hasura-T29 | Elevation of privilege | The `upload_signature` password reaches a `sh -c` command and the logs in windmill consolidation signing (`packages/windmill/src/services/consolidation/rsa.rs`, `signatures.rs`, `upload_signature_service.rs`), and harvest logs it through the `Debug` of `UploadSignatureInput` (`packages/harvest/src/routes/miru_plugin.rs`) | No control on release/10.0; the fix is in sequentech/step#3522 | Open on release/10.0 (fix in sequentech/step#3522) |

## Assumptions

- Keycloak (tenant and election-event realms, extended by keycloak-extensions) issues the only tokens Hasura accepts, and each token carries only the `x-hasura-*` claims and roles its holder should have. Only backend services hold `service-account`.
- Only windmill can write the JWKS document that Hasura fetches.
- Every deployment sets the JWT config and `HASURA_GRAPHQL_UNAUTHORIZED_ROLE=unauthorized`, uses strong secrets, disables the console and dev mode, excludes `query-log`, restricts CORS, and limits request size, depth and rate at ingress. The env files under `.devcontainer/` are for development and demos only.
- harvest is reachable only from Hasura and the documented internal callers on a private network, and authenticates and authorizes every request it receives.
- In production, the Postgres role Hasura uses is not a superuser.
- braid, the bulletin board and the ballot verifier detect tampering with keys, ballots and tallies that privileged database users could otherwise make undetected.
- The edge proxy sets the client IP and country headers.

## Review focus

1. Every non-service grant: its row filter, post-update check and column list, because tenant and event isolation rests on them.
2. Row filters and column lists of the voter role `user` on every table it can read, because voters are the largest untrusted population.
3. Permission-label enforcement for labelled admins, because labels are the only separation inside a tenant.
4. How Keycloak groups and realm configuration map to Hasura roles and claims.
5. The JWT trust anchor and how its keys are managed.
6. The Hasura to harvest hop: network reachability, authentication of forwarded requests, and per-action authorization.
7. Which roles can change election-critical data, and admin reads of the voter-ballot link.
8. Where credentials and personal data are stored, and which roles can read them.
9. Engine settings in each shipped configuration, and what logs would capture.
10. DB triggers that guard election-critical columns.
11. Permission regression tests and the CI jobs that run them.
