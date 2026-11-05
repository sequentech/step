<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# harvest threat model

harvest is the Rocket REST API behind the Hasura actions, plus the `/api/datafix` voter-registry API and two routes Keycloak calls. It runs as a server container (`Dockerfile.prod`, port 8400) behind Hasura; election managers, trustees and voters reach it through Hasura actions, while the voter registry and Keycloak call it directly. It acts with Keycloak admin credentials and direct Postgres connections, so Hasura row permissions do not apply and harvest makes its own tenant, event and label checks. Long jobs go to windmill over RabbitMQ, and some windmill code runs inside this process. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **Keycloak realms (users, credentials, roles, authentication flows, realm settings)**: harvest changes them through the Keycloak admin API. Integrity and confidentiality.
- **Election state (voting status, ballot publication, tallies, results publication)**: integrity and availability.
- **Cast ballots and their metadata**: ballot integrity and voter privacy.
- **Encrypted trustee share data and trustee configurations**: the trustees' custody of the threshold election key. Confidentiality.
- **Voter PII, encrypted (secret) profile attributes, voter passwords and Datafix PINs**: confidentiality and integrity.
- **Passwords for documents, reports, archives and the Miru PKCS#12**: confidentiality.
- **Integration credentials**: Keycloak admin, Datafix, immudb, Cloudflare and Google credentials. Confidentiality.
- **Documents and presigned MinIO URLs**: exports, reports and configuration bundles. The URLs are bearer capabilities. Confidentiality and integrity.
- **Electoral log and pgaudit records in immudb**: the audit trail. Integrity and confidentiality.
- **Shared Cloudflare zone**: availability and integrity of every tenant's voting domains.
- **WASM plugins loaded at startup**: run inside harvest with database host functions. Integrity.
- **Service logs**: confidentiality.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| Admin Hasura actions on `/` (`src/main.rs` `rocket()`, `src/routes/*.rs`, about 120 routes) | Election managers via Hasura (`../../hasura/metadata/actions.yaml`) | authenticated admin | Hasura role allow-list per action; `authorize` in each route |
| `src/routes/insert_tenant.rs` `insert_tenant`, `src/routes/delete_tenant.rs` `delete_tenant_f` | Super-admin tenant users via Hasura | authenticated admin | `authorize` without a tenant (super-admin tenant only) with `TENANT_CREATE` / `TENANT_DELETE`; `tasks_execution` record marked failed on deny |
| `src/routes/voting_status.rs`, `ballot_publication.rs`, `tally_ceremony.rs`, `tally_sheets.rs`, `results_publication.rs` | Election managers via Hasura | authenticated admin | `ELECTION_STATE_WRITE`, `PUBLISH_*`, `ADMIN_CEREMONY`, `TALLY_*`, `PUBLISH_RESULTS_WRITE`; `has_gold_permission` step-up |
| `src/routes/users.rs`, `roles.rs`, `permissions.rs`, `realm_attributes.rs`, `realm_password_policy.rs`, `set_voter_authentication.rs`, `applications.rs` `change_application_status` | Election managers via Hasura | authenticated admin | `authorize` in each route (`USER_*`, `VOTER_*`, `ROLE_*`, `USER_PERMISSION_*`, `KEYCLOAK_REALM_ATTRIBUTES_*`, `APPLICATION_WRITE`, `VOTER_SECRET_ATTRIBUTE_*`, `PERMISSION_LABEL_WRITE`) |
| `POST /verify-application` (`src/routes/applications.rs` `verify_user_application`) | keycloak-extensions voter enrollment flow, directly with a service-account token | internal service | `authorize` with `SERVICE_ACCOUNT` |
| Imports and exports: `src/routes/insert_election_event.rs` `import_election_event_f`, `export_election_event.rs`, `import_*.rs`, `export_*.rs`, `tally_sheets.rs`, `datafix_reconciliation.rs`, `trustees.rs` | Election managers via Hasura | authenticated admin | `authorize` in each route; SHA-256 `integrity_check` on import; tally sheet size cap and `verify_source_sha256` |
| Documents and reports: `src/routes/upload_document.rs`, `fetch_document.rs`, `document_password.rs`, `reports.rs`, `voter_information_letter.rs`, `manual_verification_pdf.rs`, `generate_preview_url.rs` | Election managers via Hasura | authenticated admin | `DOCUMENT_*`, `REPORT_*`, `GENERATE_PREVIEW`, `VOTER_INFORMATION_LETTER`, `VOTER_MANUALLY_VERIFY` |
| Audit reads: `src/routes/electoral_log.rs` `list_electoral_log`, `src/routes/immudb_log_audit.rs` `list_pgaudit` | Election managers via Hasura | authenticated admin | `LOGS_READ` |
| Scheduling, phone blacklist and certificate authorities: `src/routes/scheduled_event.rs`, `election_dates.rs`, `phone_blacklist.rs`, `*_certificate_authority.rs` | Election managers via Hasura | authenticated admin | `authorize` in each route |
| Integrations: `src/routes/custom_urls.rs`, `limit_access_by_countries.rs`, `google_meet.rs`, `miru_plugin.rs` | Election managers via Hasura | authenticated admin | `ELECTION_EVENT_WRITE`, `CLOUDFLARE_WRITE`, `GOOGLE_MEET_LINK`, `MIRU_*` |
| `POST /plugin` (`src/routes/plugins.rs` `plugin_routes`) | Hasura role `admin-user` | authenticated admin | Hasura role; plugin-side calls to the host `authorize` function |
| Trustee routes: `src/routes/keys_ceremony.rs` `get_private_key`, `check_private_key`; `src/routes/tally_ceremony.rs` `restore_private_key` | Trustees via Hasura | trustee | `TRUSTEE_CEREMONY`; ceremony and trustee state checked in windmill |
| Voter routes: `src/routes/insert_cast_vote.rs`, `create_ballot_receipt.rs`, `voter_electoral_log.rs`, `support_materials.rs`, `ballot_files.rs`, `results_publication.rs` `resolve_results_publication` / `fetch_results_artifact` | Voters via Hasura role `user` | authenticated voter | sequent-core `authorize_voter_election` / `authorize_voter_event`; results reads checked in windmill `services/results_publication.rs` |
| `/api/datafix/*` (`src/routes/api_datafix.rs`) | External voter registry, directly | untrusted | Client-credentials grant at the tenant realm; `DATAFIX_ACCOUNT`; per-voter lock; electoral-log audit of each operation; Datafix catchers in `src/routes/error_catchers.rs` |
| `GET /election-event/<id>/certificate-authorities/pem` (`src/routes/get_certificate_authorities_pem.rs` `get_cas_pem`) | Keycloak truststore SPI; anyone who reaches harvest | untrusted | UUID parse; returns public CA certificates only |
| Process start (`src/main.rs` `rocket()`) | Operator | operator | Operator-controlled configuration (deployment) |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| harvest-T1 | Spoofing | A caller reaches harvest without going through Hasura and presents forged or replayed bearer claims | Token validation and the action role allow-list in Hasura (`../../hasura/metadata/actions.yaml`); network isolation of the harvest port (deployment) | Not verified |
| harvest-T2 | Spoofing | The voter registry on `/api/datafix/*` is impersonated | Keycloak checks the client credentials (client-credentials grant at the tenant realm); `authorize` with `DATAFIX_ACCOUNT`; uniform `datafix_*` catchers in `src/routes/error_catchers.rs` | Partial |
| harvest-T3 | Elevation of privilege | An admin of one tenant reads or changes another tenant's data | `authorize` tenant check in each route (sequent-core `src/services/authorization.rs`) | Partial |
| harvest-T4 | Elevation of privilege | An admin limited to some elections by permission labels acts on other elections of the tenant | Permission-label checks (windmill `validate_permission_labels`, sequent-core `decode_permission_labels`) | Partial |
| harvest-T5 | Elevation of privilege | A route accepts an admin whose granular permissions should not allow the operation | `Permissions` list passed to `authorize` in each route; Hasura role allow-list per action | Partial |
| harvest-T6 | Tampering | A request identifier that harvest passes to a downstream system selects a resource outside the caller's tenant or event | `authorize` tenant and permission checks before the downstream call | Not verified |
| harvest-T7 | Tampering | A tenant admin changes shared Cloudflare configuration in a way that affects other tenants | `ELECTION_EVENT_WRITE` for custom URLs; `CLOUDFLARE_WRITE` for country limits | Partial |
| harvest-T8 | Tampering | A stolen or idle admin session changes election state | `has_gold_permission` step-up (acr gold, authentication under 60 s) | Partial |
| harvest-T9 | Information disclosure | A trustee downloads another trustee's share data, or downloads it again after the key check | `TRUSTEE_CEREMONY`; windmill `validate_private_key_download` under the ceremony lock; generic `private_key_download_internal_error` | Partial |
| harvest-T10 | Information disclosure | The trustee configuration export exposes every trustee's keys at once | `TRUSTEES_EXPORT` and a `tasks_execution` record | Partial |
| harvest-T11 | Information disclosure | Voter PII or encrypted profile attributes are read without the dedicated permission | `VOTER_SECRET_ATTRIBUTE_READ` and an audit entry before decryption in `src/routes/users.rs` `reveal_voter_secret_attribute`; `ensure_secret_attributes_not_queried`; windmill `redact_user`; the same permission re-checked when fetching a document with secret attributes or its password | Partial |
| harvest-T12 | Information disclosure | Secrets or PII reach the logs | Secret fields kept out of route logs (for example `src/routes/document_password.rs`, `realm_attributes.rs`); access-controlled log storage (deployment) | Partial |
| harvest-T13 | Information disclosure | `/miru/upload-signature`: the derived `Debug` of `UploadSignatureInput` logs the PKCS#12 password, which windmill then formats into an `sh -c` command | None on release/10.0 (`src/routes/miru_plugin.rs` `upload_signature`) | Open on release/10.0 (fix in sequentech/step#3522) |
| harvest-T14 | Information disclosure | A tenant admin reads audit-log entries of other tenants | `LOGS_READ` | Not verified |
| harvest-T15 | Information disclosure | Internal error details (SQL, hosts, Keycloak responses) are returned to callers | Generic errors in `src/routes/document_password.rs`, `keys_ceremony.rs` `get_private_key`, `tally_ceremony.rs` `tally_response_error`, `ballot_files.rs`; generic catchers in `src/routes/error_catchers.rs` | Partial |
| harvest-T16 | Elevation of privilege | User-supplied content reaches server-side template and PDF rendering in windmill | harvest only queues the rendering tasks; rendering lives in windmill and sequent-core | Partial |
| harvest-T17 | Elevation of privilege | A faulty or tampered plugin, or a caller of a plugin route, misuses the database host functions that plugins have inside harvest | Hasura `admin-user` role on `call_plugin_route`; host `authorize` binding; WASI context with only inherited stdio and no preopened directories or environment (windmill `services/plugins_manager/plugin.rs` `Plugin::init_plugin_from_wasm_bytes`) | Partial |
| harvest-T18 | Denial of service | `POST /plugin` with non-object `data` panics the handler | None on release/10.0 (`src/routes/plugins.rs` `plugin_routes`) | Open on release/10.0 (fix in sequentech/step#3522) |
| harvest-T19 | Denial of service | Oversized or crafted uploads and imports, or request floods on directly exposed routes, exhaust harvest or Keycloak | Rocket default body limits (no `Rocket.toml`); `MAX_TALLY_SHEET_IMPORT_BYTES` in `src/routes/tally_sheets.rs` | Partial |
| harvest-T20 | Repudiation | An admin or registry action leaves no attributable record | `audit_secret_attributes` / `post_voter_secret_attribute_audit`; Datafix `audit_inbound_operation`; `tasks_execution` rows with the executor in `src/routes/insert_tenant.rs`, `delete_tenant.rs`, `delete_election_event.rs`, `trustees.rs`, `export_ballot_publication.rs` | Partial |
| harvest-T21 | Tampering | The IP and country stored with a cast vote are spoofed, hiding abuse from per-IP monitoring | The edge proxy sets the client IP and country headers (deployment) | Not verified |
| harvest-T22 | Tampering | A voter casts a ballot they are not eligible to cast, or outside the voting period, channel or revote limit | `authorize_voter_election` with `CAST_VOTE` in `src/routes/insert_cast_vote.rs`; voting status checked in windmill before the insert; revote limit in a database trigger | Not verified |
| harvest-T23 | Information disclosure | A voter reads data about other voters | `authorize_voter_election` / `authorize_voter_event` on voter routes | Not verified |
| harvest-T24 | Information disclosure | Integration credentials that harvest uses leak or are misused | Credentials kept out of the source code | Not verified |
| harvest-T25 | Tampering | An election-event or trustee archive is read or altered by someone who holds the file but not its password | Password-based archive encryption; SHA-256 `integrity_check` and `check_only` dry run on import (`src/routes/insert_election_event.rs`) | Partial |
| harvest-T26 | Information disclosure | Passwords and key material outlive the request that handled them | PDF passwords kept in the vault (`src/routes/voter_information_letter.rs`, `document_password.rs`) | Partial |
| harvest-T27 | Tampering | The runtime image carries untrusted third-party code, or the container runs with more privilege than harvest needs | Multi-stage build with base images pinned by tag and `cargo build --locked` (`Dockerfile.prod`) | Partial |
| harvest-T28 | Information disclosure | Anyone who reaches harvest lists an event's certificate authorities without a token | `src/routes/get_certificate_authorities_pem.rs` `get_cas_pem` parses the event id as a UUID and returns only public CA certificates | Accepted (by design: Keycloak's truststore SPI fetches the bundle without credentials, and the certificates are public) |

## Assumptions

- Hasura validates the Keycloak JWT and the action role allow-list on every action before it forwards the request.
- Only Hasura and Keycloak can reach harvest's port. The exception is `/api/datafix/*`, which sits behind controlled ingress (deployment).
- Keycloak and the keycloak-extensions mappers put correct Hasura claims in tokens.
- `SUPER_ADMIN_TENANT_ID` is set and its realm is tightly administered.
- The edge proxy sets the client IP and country headers.
- RabbitMQ and the windmill workers are inside harvest's trust boundary. windmill owns the parsing, rendering and outbound calls done in tasks.
- Postgres, immudb, MinIO, Keycloak admin and Cloudflare credentials come from the environment and are protected by the deployment. The secrets backend protects the vault master secret.
- The pgaudit ingestion that fills the immudb audit tables runs outside this repository.
- Only operators can change the plugins harvest loads, and plugins are trusted code.
- Log storage is access-controlled, and `LOG_LEVEL` is not `debug` in production.

## Review focus

1. Input validation, including values that harvest passes to downstream systems.
2. Tenant, election-event, voter and permission-label scoping in every route.
3. The permission each route requires for the operation it performs.
4. Network exposure of harvest and how it authenticates the requests it receives.
5. Secret and PII handling in logs and error responses.
6. Password and key handling for archives, reports and signing.
7. Integrations shared by all tenants, and the handling of their credentials.
8. User-supplied content that reaches server-side rendering.
9. Plugin supply chain and runtime limits.
10. Resource limits on archive and document processing done in the harvest process, and flood resistance of the directly exposed routes.
11. Runtime image hardening and supply chain.
