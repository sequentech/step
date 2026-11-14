<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# windmill threat model

windmill is the platform's background worker. It runs in worker containers as Celery consumers of RabbitMQ queues (`src/bin/main.rs`) and as a periodic scheduler (`src/bin/beat.rs`). harvest also links it as a library and calls it in-process for cast-vote insertion, trustee key downloads, user queries, Datafix voter updates, voter-facing log views, Cloudflare changes, SBEI signature uploads and plugin routes. Its jobs cover imports and exports, key and tally ceremonies on the b3 board, report and PDF rendering, voter communications, scheduled voting-period changes, results publication, Miru transmission, WASM plugins and electoral-log ingestion. Election managers, voters, trustees and Datafix clients reach it only through harvest, Hasura and the broker. windmill connects to the Hasura and Keycloak databases directly, holds the platform master secret, writes the JWKS that Hasura trusts and signs board and audit messages, so a flaw here can cross tenants and affect ballot secrecy, tally integrity, the audit trail and token trust. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **Master secret and the `secret` table**: the master secret encrypts every stored secret and derives the voter secret-attribute keys. Confidentiality and availability (losing it makes every stored secret unreadable).
- **Protocol Manager signing keys**, one per board: they sign b3 configuration and ballot messages and every electoral-log entry. Confidentiality and integrity.
- **Trustee configs and encrypted key shares**: the threshold decryption capability. Confidentiality.
- **Cast ballots, decrypted plaintexts and results**: integrity and ballot secrecy.
- **Voter PII, credentials, PINs and secret attributes**: confidentiality.
- **Election schedule and voting status**: `scheduled_event` rows and the status changes the beat loop applies. Integrity and availability.
- **Electoral log in immudb**: the audit trail. Integrity, non-repudiation and voter privacy.
- **Aggregated JWKS (`certs.json`)**: the keys Hasura trusts. Integrity.
- **Plugins and templates in object storage**: WASM components under `plugins/` and report templates that the worker runs or renders. Integrity.
- **Exports, reports and documents**: archives and PDFs with keys, voter lists, secret attributes, results and logs. Confidentiality.
- **Transmission packages and SBEI signing material**: results sent to Miru CCS servers, PKCS#12 keys and their passwords. Integrity and confidentiality.
- **Service and third-party credentials**: database, AMQP, S3, immudb and b3 credentials, Keycloak client secrets, secrets-backend tokens, Cloudflare, SMTP/SMS, Google and VoterView credentials. Confidentiality.
- **Secret-export grants**: the `secret_export_authorization` annotation on `tasks_execution` rows. Integrity.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| 58 Celery tasks registered in `src/services/celery_app.rs` `generate_celery_app`, consumed by `src/bin/main.rs` | Any holder of broker credentials: harvest, beat, chained windmill tasks, the `Produce` CLI | internal service | Broker credentials (`AMQP_ADDR`); queues prefixed with `ENV_SLUG` (`get_queue_name`) |
| Beat jobs in `src/bin/beat.rs`: `review_boards`, `scheduled_events`, `scheduled_reports`, `review_cast_votes`, `electoral_log_batch_dispatcher` | Internal timer | internal service | Not externally callable |
| Export tasks (`src/tasks/export_*.rs`) | harvest after its permission check | authenticated admin (via harvest) | harvest permission checks |
| Import tasks (`src/tasks/import_*.rs`) | Admin uploads via harvest | authenticated admin | harvest permission checks |
| Electoral-log queue (`src/tasks/electoral_log.rs`) | Producers on the broker: the Keycloak event listener, harvest, windmill | internal service | Broker credentials |
| Cast-vote insertion (`src/services/insert_cast_vote.rs`, in-process) | harvest on voter requests | authenticated voter | Ballot and election-state checks |
| Trustee key download and check (`src/services/ceremonies/`, in-process) | harvest on trustee requests | trustee | Ceremony state checks under a ceremony lock |
| Ballot receipts, reports and document rendering (`src/tasks/`) | Voters (receipts) and admins (templates, documents) via harvest | authenticated voter; authenticated admin | Receipt only for the voter's own cast ballot |
| Cloudflare custom URLs and country limits (in-process) | harvest routes | authenticated admin | harvest permission checks |
| Miru transmission and SBEI signature tasks (`src/tasks/miru_plugin_tasks.rs`) | Admins and SBEI members via harvest | authenticated admin | harvest permission checks |
| Datafix voter API (`src/services/datafix/`, in-process) | External Datafix client via harvest | authenticated service client | Event resolved from the client's Datafix id, ambiguous ids rejected (`src/services/datafix/utils.rs`); per-voter lock |
| Voter-facing log view (in-process) | Voters via harvest | authenticated voter | The event's log-view policy |
| WASM plugins (`src/services/plugins_manager/`, `src/tasks/plugins_tasks.rs`) | Plugin binaries from object storage; harvest plugin routes and broker | operator (binaries); authenticated admin (calls) | Deployment (see Assumptions) |
| Probe `src/services/probe.rs` `setup_probe` (`/live`, `/ready`, `{APP}_PROBE_ADDR`, default `0.0.0.0:3030`) | Anyone on the network path | untrusted | None; relies on network policy |
| Operator tools (`external-bin/`) | Operators with shell access | operator | Credentials from config files |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| windmill-T1 | Spoofing, Elevation of privilege | A party that can publish to RabbitMQ sends forged task messages, and the worker acts on them. | Broker credentials (`src/services/celery_app.rs` `create_connection`); queues prefixed per environment. | Partial |
| windmill-T2 | Elevation of privilege, Information disclosure | Direct SQL does not go through Hasura row permissions, so a query without tenant or event scoping reads or writes another tenant's data. | Lookups by tenant and id: `src/postgres/election_event.rs` `get_election_event_by_id`, `src/postgres/document.rs` `get_document`, `src/postgres/secret.rs`. `src/services/datafix/utils.rs` `get_event_id_and_datafix_annotations` rejects ambiguous matches. | Partial |
| windmill-T3 | Tampering, Denial of service | The election schedule, or the status changes the beat loop applies from it, is tampered with or disrupted. | Deployment (see Assumptions). | Not verified |
| windmill-T4 | Tampering | Forged, altered or ineligible ballots are inserted through the cast-vote path. | Ballot and election-state checks on insert. | Partial |
| windmill-T5 | Information disclosure | Keys, passwords, tokens or voter PII reach logs, traces or broker messages. | Secret arguments skipped in tracing spans in, for example, `src/services/document_password.rs` and `src/tasks/edit_user.rs`. `external-bin/entrypoint.sh` disables core dumps. | Partial |
| windmill-T6 | Elevation of privilege, Information disclosure | Consolidation signing built `sh -c` command lines containing the user-supplied PKCS#12 password and logged it (`src/services/consolidation/rsa.rs`, `src/services/consolidation/signatures.rs`, `src/services/consolidation/upload_signature_service.rs`); `run_shell_command` in `../sequent-core/src/signatures/shell.rs` logged the full command. | None on this branch. | Open on release/10.0 (fix in sequentech/step#3522) |
| windmill-T7 | Spoofing, Tampering | The CCS transmission upload disabled TLS certificate checks (`src/services/consolidation/send_transmission_package_service.rs`). | The package is encrypted to the server's ECIES key and signed. | Open on release/10.0 (fix in sequentech/step#3522) |
| windmill-T8 | Spoofing | The manual verification link did not URL-encode `userId` or `redirectUri` (`src/services/reports/manual_verification.rs`). | None on this branch. | Open on release/10.0 (fix in sequentech/step#3522) |
| windmill-T9 | Denial of service | `execute_plugin_task` panicked when `data` was not a JSON object (`src/tasks/plugins_tasks.rs`). | None on this branch. | Open on release/10.0 (fix in sequentech/step#3522) |
| windmill-T10 | Elevation of privilege, Denial of service | A malicious or faulty WASM plugin tampers with data or disrupts the worker. | Deployment (see Assumptions). | Not verified |
| windmill-T11 | Spoofing, Elevation of privilege | The JWKS that Hasura trusts is tampered with, so someone obtains tokens Hasura accepts. | Deployment (see Assumptions). | Not verified |
| windmill-T12 | Information disclosure | Exports, reports and documents expose keys, voter PII, secret attributes or results. | Voter letters are PDF-encrypted (`src/services/pdf_encryption.rs` `encrypt_pdf`). | Partial |
| windmill-T13 | Elevation of privilege, Information disclosure | HTML rendered to PDF in headless Chromium compromises the renderer or what it can reach. | Deployment (see Assumptions). | Not verified |
| windmill-T14 | Tampering, Denial of service | Admin-supplied imports (event archives, voter CSVs, tenant configs, ES&S XML) carry hostile content that tampers with data or disrupts the worker. | `src/services/sql_utils.rs` `escape_sql_identifier`, `escape_sql_literal`, and `assert_standard_conforming_strings` at pool creation (`src/services/database.rs`). UUIDs parsed before COPY (`src/postgres/candidate.rs`). Sort fields allowlisted (`src/services/users.rs` `get_sort_clause_and_field_param`). `roxmltree` rejects DTDs (`src/services/ess_xml_converter.rs`). | Partial |
| windmill-T15 | Repudiation, Tampering | Audit events are forged or lost between producers and immudb. | Queue access requires broker credentials. | Partial |
| windmill-T16 | Information disclosure, Denial of service | The master secret or stored secrets leak, or one key compromise exposes every tenant. | Stored secrets are encrypted at rest. The Hasura `secret` table has no role permissions. Secret-attribute keys are derived per tenant, event, user and attribute (`src/services/voter_secret_attributes.rs` `derive_key`). | Partial |
| windmill-T17 | Information disclosure, Elevation of privilege | Trustee key shares or configs leave trustee control. | Key downloads are allowed only while the ceremony is in progress and before the key check. | Partial |
| windmill-T18 | Spoofing, Tampering, Information disclosure | Outbound integrations (Cloudflare, Miru CCS, VoterView, Google Calendar, SMTP/SMS) are misused or expose their credentials. | Email and SMS transports picked from a fixed list (`src/services/providers/email_sender.rs`, `src/services/providers/sms_sender.rs` `from_transport_name`). Database TLS with a CA file for `Prefer` / `Require` (`src/services/database.rs`). | Not verified |
| windmill-T19 | Information disclosure, Denial of service | The unauthenticated probes are used to learn about the worker or to load it. | Bind address and paths configurable per binary (`src/services/probe.rs` `setup_probe`); responses carry only a status. | Partial |
| windmill-T20 | Tampering | Tally or published results are altered. | braid proofs, checked independently with the braid verifier (`../braid`). | Not verified |
| windmill-T21 | Information disclosure | The voter-facing log view discloses personal data. | Shown only when the event's policy enables it (`ShowCastVoteLogs`). | Not verified |

## Assumptions

- RabbitMQ is reachable only by platform services with non-default credentials and TLS where traffic leaves the host; anyone who can publish, including the Keycloak event listener, is trusted like harvest.
- harvest authenticates and authorizes every request before it enqueues a task or calls windmill in-process.
- Hasura permissions enforce tenant isolation, and only `service-account` writes `tasks_execution`.
- Write access to object storage, including `certs.json`, plugins and templates, is limited to platform services and operators. Plugins are reviewed code.
- Operators restrict access to logs and to the secrets backend.
- The deployment isolates the PDF renderer and restricts what it can reach.
- Probe ports are not exposed outside the cluster, and the database `ssl_mode` is `Require` in production.
- braid trustees protect their own keys, and b3 and immudb are reachable only from inside the platform.

## Review focus

1. The broker boundary.
2. Handling of secrets and personal data across the crate.
3. Tenant and event scoping of direct SQL and of beat jobs.
4. Keycloak integration and JWKS publication.
5. HTML-to-PDF rendering and renderer isolation.
6. Export and import pipelines.
7. Cloudflare and other outbound integrations.
8. The plugin host.
9. Subprocess invocation.
10. Electoral-log ingestion and the voter-facing log view.
11. Master-secret storage and lifecycle.
12. Tally and results publication.
