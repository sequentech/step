<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# step-cli threat model

step-cli is the command-line tool for election administration: a single Rust binary (`src/main.rs`, with the `step` and `load` command groups) that election managers, trustees, CI jobs and load-test operators run on their own hosts, in the devcontainer, from the `Dockerfile` image and in the load-test images. It listens on no port. API commands log in to the tenant's Keycloak realm, then call Hasura and harvest with the user's own JWT, so all authorization happens on the server. A few test-data and export commands connect straight to Postgres or immudb with whatever credentials the operator supplies. Trustees use it to download, store and later present their encrypted key share. The `load` group provisions a synthetic election and drives load tests against it. step-cli holds no privilege of its own: the damage a misuse can do depends on the session it stores, the files it writes and the stores it can reach. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **Operator session**: access token, refresh token, client id and client secret in `<exe dir>/config/configuration.json` (`src/types/config.rs` `ConfigData`). Confidentiality: they grant the user's tenant-admin or trustee rights. Integrity of the stored endpoints.
- **Secrets passing through the process**: Keycloak password and client secret, voter passwords, the immudb password, the synthetic load password. Confidentiality.
- **Trustee encrypted key share**: the blob that `complete-key-ceremony` downloads once and `confirm-key-tally` presents at tally. Availability (the tally needs it) and confidentiality.
- **Exports and downloads**: election event archives (voters, applications, logs, bulletin board, tally) and their generated password, `tally.tar.gz`, tally-sheet sources, cast-vote CSVs linking voter pseudonyms to ballot hashes. Confidentiality before publication; integrity of what the operator relies on.
- **Voter census and credentials**: generated or imported CSVs with PII and plaintext or hashed passwords. Confidentiality.
- **Direct store credentials**: `KC_DB_*`, `HASURA_PG_*`, `IMMUDB_*`, the load audit DSN. Confidentiality: they give direct access to the stores, outside API authorization.
- **Backend records reachable through those credentials**: `cast_vote` rows, applications, Keycloak users, electoral-log entries. Integrity of ballots and of the audit trail.
- **Load-test material**: settings, synthetic census, encrypted ballot shards, digests and reports under the run directory (`src/load/`). Integrity of results; confidentiality of the shared synthetic password.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| `step config`, `step refresh-token` (`src/commands/configure.rs` `Config`, `src/commands/refresh_token.rs` `Refresh`) | Anyone with a tenant-realm account and the client credentials | operator | Keycloak login against realm `tenant-<id>` (`src/utils/keycloak.rs`); session written by `src/utils/read_config.rs` `write_config` |
| Admin API commands: `create-*`, `delete-*`, `update-*`, `publish`, imports, exports, key-ceremony, tally, preview and results commands (`src/main.rs` `StepCommands`, `src/commands/*.rs`) | Holder of the stored session | authenticated admin | Bearer JWT from the stored session; no Hasura admin secret; Hasura permissions and harvest permission checks on the server |
| `complete-key-ceremony`, `confirm-key-tally` (`src/commands/complete_key_ceremony.rs` `complete_ceremony`, `src/commands/confirm_tally_ceremoney_key.rs` `confirm_key`) | Trustee user | trustee | harvest `TRUSTEE_CEREMONY` and the JWT `trustee` claim; server-side download and presentation checks |
| Document transfer: `upload-document`, `download-document`, `download-tally-results`, `tally-sheet` imports and source downloads (`src/utils/upload_file.rs`, `src/commands/tally_sheet.rs`) | Holder of the stored session | authenticated admin | Presigned URLs issued by harvest after `DOCUMENT_UPLOAD` / `DOCUMENT_DOWNLOAD`; uploads requested as private objects |
| `duplicate-votes`, `create-applications`, `create-electoral-logs` (`src/commands/duplicate_votes.rs`, `create_applications.rs`, `create_electoral_logs.rs`) | Anyone with the Keycloak DB, Hasura DB and immudb credentials in the environment | operator | Store credentials supplied by the operator; targets read from `<working_directory>/external_config.json`; parameterized SQL |
| `export-cast-votes` (`src/commands/export_cast_votes.rs` `ExportCastVotes`) | Anyone with immudb credentials | operator | immudb credentials and board supplied by the operator; read-only query |
| `render-template`, `generate-voters`, `hash-password`, `tally-sheet convert-ess-xml` | Local user | operator | None: local files only |
| `load init/check/prepare/run/report/image/reference` and hidden `setup` / `encrypt` (`src/load/mod.rs` `Command`) | Operator with a tenant session and Docker or Kubernetes access | operator | `src/load/config.rs` `Settings::validate`; `src/load/input.rs` `Input::validate`; session, tenant and URL checks before provisioning |
| Load engines (`src/load/worker.rs` `node`, `shard`; `step-load-worker` built from `../voting-load`) | Coordinator, Docker or Kubernetes job | internal service | Cleared environment with an allowlist; config and shard digests |
| Responses from Keycloak, Hasura, harvest, object storage, Postgres and immudb | Remote services | internal service | `reqwest` default TLS verification; typed GraphQL deserialization; `Path::file_name` on server-supplied file names |
| `scripts/*.py` load-test helpers | Operator | operator | `scripts/load_test_common.py` `run_step` redacts login credentials from captured output |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| step-cli-T1 | Elevation of privilege | A CLI user acts beyond their roles or outside their tenant through the server actions the CLI calls | No command sends the Hasura admin secret; `x-hasura-role` only selects among the JWT's allowed roles; authorization is left to the server (Hasura permissions, harvest permission checks) | Partial |
| step-cli-T2 | Spoofing | An attacker with a stolen admin password opens a CLI session | Login scoped to the tenant realm; harvest repeats `has_gold_permission` before ballot publication and voting-status changes; login strength otherwise depends on the realm's authentication settings (deployment) | Not verified |
| step-cli-T3 | Spoofing | Another local user reads, alters or reuses the stored session | `src/utils/read_config.rs` `write_config` writes the session owner-only with an atomic rename; password not stored | Partial |
| step-cli-T4 | Information disclosure | Operator secrets leak on the operator host or into CI logs | Load secrets read from environment variables (`src/load/config.rs` `password_env`, `load report --dsn-env`); load-test scripts redact login credentials from captured output; session tokens are not printed | Partial |
| step-cli-T5 | Information disclosure | The trustee's key share is read, replaced or lost on the trustee host, weakening custody or blocking the tally | Server refuses a new download once the trustee has checked the key; the CLI stores the share before the check and checks the stored copy on retry; the server checks presented shares | Partial |
| step-cli-T6 | Information disclosure | Exports, tally archives, cast-vote CSVs, census files or the secrets protecting them are exposed on the operator host | Export archives can be encrypted with a generated password; the load census holds hashes, not plaintext passwords | Partial |
| step-cli-T7 | Tampering | A presigned URL leaks, or a document is read, swapped or altered between upload and use | `reqwest` TLS verification; `src/utils/upload_file.rs` requests private objects; harvest issues URLs per document after authorization | Partial |
| step-cli-T8 | Tampering | Whoever holds direct store credentials adds ballots, applications or electoral-log entries that no voter or service produced | Store credentials must be supplied by the operator; targets read from `external_config.json`; parameterized inserts | Partial |
| step-cli-T9 | Information disclosure | Credentials or voter data on direct store connections are intercepted in transit | Network isolation (deployment) | Not verified |
| step-cli-T10 | Elevation of privilege | A load test affects a real election, or synthetic resources outlive the test | Provisioning requires the session to match the target tenant, GraphQL URL and Keycloak URL; Kubernetes password held in a Secret (`src/load/executor.rs` `job`) | Partial |
| step-cli-T11 | Information disclosure | Administrator credentials reach load workers or remote executors | `src/load/worker.rs` `environment` (`env_clear`, `ENGINE_ENV` allowlist, only `LOAD_CONFIG` and `LOAD_PASSWORD`); Docker gets `-e <name>` without the value; `src/load/executor.rs` `kubectl` sends manifests on stdin; setup runs in a subprocess with a private `setup.log` (`src/load/coordinator.rs` `prepare`); test `indexed_jobs_have_no_retries_or_embedded_passwords` | Mitigated |
| step-cli-T12 | Tampering | Load inputs, ciphertext shards or the bundled runtime change between prepare and run, or runs get mixed | `src/load/files.rs` `claim_directory`, `create` (0700/0600, `create_new`), `digest`; `src/load/worker.rs` `shard` checks `config_sha256` and each shard's `.sha256`; `src/load/mod.rs` `runtime` refuses a modified cache. Digests sit in the run directory, so they catch accidental change; the directory mode is the access control | Partial |
| step-cli-T13 | Tampering | Worker-sourced text injects markup or script into the HTML load report | `src/load/presentation.rs` `escape` on every text value; the rest of the report is numbers and fixed labels | Mitigated |
| step-cli-T14 | Information disclosure | Voter passwords produced by the CLI are hashed weakly or share one credential | `src/commands/hash_passwords.rs` `run_hash_password` (PBKDF2-HMAC-SHA256, random 32-byte salt per row, 600000 iterations by default, plaintext column dropped); `src/load/census.rs` `generate` (ring `SystemRandom` salt, no plaintext column; one salt and hash per census because every synthetic voter shares the load password by design); `src/commands/generate_voters.rs` `VoterPasswordPolicy::RandomNumeric` for per-voter PINs; iteration count and password policy are operator choices | Partial |
| step-cli-T15 | Repudiation | An administrative or trustee action taken through the CLI cannot be attributed to a person | Every API call carries the user's own JWT; harvest records the executor on task executions (windmill `tasks_execution::post`); each trustee's key download and check is recorded in the election's logs | Partial |
| step-cli-T16 | Elevation of privilege | Whoever can edit load settings or a run directory runs code as the operator | Run directories claimed 0700 (`src/load/files.rs` `claim_directory`); `Settings::validate` checks target URLs; executables (`runtime.k6`, `runtime.node`, `runtime.opener`, `preparation.publication_preparer`) and the container image are named by the operator | Accepted (settings files and run directories are operator-owned code inputs) |

## Assumptions

- **Hasura and harvest** authorize every operation the CLI calls against the caller's tenant and permissions; step-cli adds no authorization of its own.
- **harvest and windmill** stop key-share downloads once a trustee has checked the key, and check the shares trustees present.
- **Keycloak** tenant realms enforce strong authentication for administrator accounts, CLI logins included, and keep token lifetimes short.
- **Operators** run the CLI on a host and account they control, protect the files it reads and writes, and delete exports and key shares once the election is closed.
- **Operators** configure HTTPS URLs for the API, Keycloak and object storage.
- **Operators** use the test-data commands only in test environments, and database and immudb ports stay on a private network.
- **Object storage** issues short-lived, single-object presigned URLs over HTTPS.
- **Load-test operators** target disposable tenants or events, treat settings files and run directories as code, and remove Kubernetes Secrets and volumes after a run.
- **electoral-log and immudb** provide the audit trail's tamper evidence.

## Review focus

- Server-side authorization of the operations the CLI calls.
- Local handling of sessions, key shares, exports and other secrets on operator and trustee hosts.
- Document upload and download through presigned URLs.
- The trustee key-share flow from download to tally.
- Direct-store commands and the credentials they use.
- Authentication strength of the CLI login in tenant realms.
- Load provisioning guardrails, settings handling and cleanup of synthetic resources.
