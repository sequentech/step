<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# sequent-core threat model

sequent-core is the shared Rust library most of the platform is built on. Compiled to WASM, it runs in the voter's browser (voting-portal), in admin-portal and in the public ballot-verifier. There it encodes, encrypts, hashes and signs ballots, and re-checks audited ones. Linked natively, it runs inside the harvest and windmill containers and in velvet. There it decodes decrypted plaintexts at tally, renders reports and PDFs, calls Keycloak and S3/MinIO, provides the Rocket request guards that harvest uses, and drives the external ECIES tool for results transmission. Cargo features (`wasm`, `reports`, `keycloak`, `s3`, `sqlite`, `signatures`, `plugins_wit`, `probe`) decide which of these each consumer gets. A flaw here affects ballot secrecy and integrity where ballots are created, and every service that links the crate. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **Vote plaintext and encryption randomness**: held in the auditable ballot in the voter's browser. Confidentiality: together with the ciphertext they reveal the vote and turn the ballot into a receipt.
- **Election public key**: carried in the ballot style (`PublicKeyConfig`). Integrity: a wrong key breaks ballot secrecy.
- **Ballot tracker (`ballot_id`) and ballot-style hash**: integrity. Voters and verifiers use them to check that a ballot was recorded as cast.
- **Ephemeral voter signing key**: one per ballot. Integrity of the binding between ballot content, `ballot_id` and `election_id`.
- **Decrypted plaintexts and write-in text at tally**: confidentiality of individual votes; integrity and availability of results.
- **ACM ECIES key pair and transmission-package passwords**: confidentiality. They sign and protect official results transmissions.
- **Keycloak service credentials (`KEYCLOAK_CLIENT_SECRET`, `KEYCLOAK_ADMIN_CLIENT_SECRET`), the cached master-realm admin token, Datafix client credentials and the tokens obtained with them**: confidentiality. They grant IdP administration across realms or tenant API access.
- **Bearer tokens and JWT claims**: integrity, because tenant, role and area authorization depend on them; confidentiality, because they carry voter and admin PII.
- **Voter PII and credentials** (Keycloak users and attributes, IP and country from request headers, generated passwords): confidentiality.
- **Realm secrets** (Smart Link shared secret, generated client secrets): confidentiality. They let the holder sign voter logins.
- **S3 credentials (`AWS_S3_ACCESS_KEY`, `AWS_S3_ACCESS_SECRET`) and presigned URLs**: confidentiality and integrity of every tenant's documents and public assets.
- **Rendered report HTML and PDFs**: confidentiality (voter letters, credentials, decoded ballots) and integrity (official results and evidence).
- **Templates and plugin binaries**: integrity, because they become rendered output or code running against the databases.
- **Keycloak realm configuration applied on import**: integrity. It decides clients, redirect URIs and token claims.
- **Renderer credentials**: confidentiality.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| `src/wasm/wasm.rs` `encrypt_decoded_contest_js`, `encrypt_decoded_multi_contest_js` | voting-portal in the voter's browser | authenticated voter (the client is under the voter's control) | `src/encrypt.rs` `votable_decoded_contests`, `validate_ballot_level_flags`; codec capacity checks; proof self-check in `encrypt_plaintext_candidate` |
| `src/wasm/wasm.rs` `to_hashable_ballot_js`, `hash_auditable_ballot_js`, `sign_hashable_ballot_with_ephemeral_voter_signing_key_js` (and multi-ballot variants) | voting-portal | authenticated voter | `TYPES_VERSION` check; strict Base64 and Borsh decoding |
| `src/wasm/wasm.rs` `verify_auditable_ballot_ciphertext_js`, `verify_ballot_signature_js`, `decode_auditable_ballot_js` (and multi-ballot variants) | ballot-verifier, voting-portal | untrusted (anyone holding an auditable ballot) | re-encryption with an exact contest-set match; `ciphertext_check_result` fails closed; decoding is a separate call |
| `src/encrypt.rs` `hash_ballot`, `hash_multi_ballot`; `src/ballot.rs` `verify_ballot_signature`; `src/multi_ballot.rs` `verify_multi_ballot_signature` | windmill cast path, on voter-submitted ballots | untrusted input, internal-service caller | strict deserialization; Ed25519 verification over `get_ballot_bytes_for_signing` |
| `src/ballot_codec/`, `src/plaintext.rs` | windmill tally and velvet, on decrypted plaintexts | untrusted content (a voter chooses the 30 encrypted bytes) | length and range checks; `raw_ballot.rs` `decode_from_raw_ballot` records encoding problems as `invalid_errors` |
| `src/services/connection.rs` `FromRequest` for `AuthHeaders`, `JwtClaims`, `DatafixClaims`, `UserLocation` | harvest HTTP routes | untrusted network input behind Hasura or the ingress | `src/services/authorization.rs` `authorize`, `authorize_voter_election`, `authorize_voter_event`; Datafix credentials exchanged at Keycloak; network isolation (deployment) |
| `src/services/keycloak/` `KeycloakAdminClient` user, group, role, permission and realm-attribute operations | harvest and windmill acting for admins | authenticated admin (authorization done by the caller) | `realm_attributes.rs` `validate_realm_attributes`, `redacted_attributes` |
| `src/services/keycloak/realm.rs` `upsert_realm`, `replace_realm_ids`, `get_realm` | windmill tenant and election-event import and export | authenticated admin (import file) | typed deserialization; portal client URLs rewritten from env; tenant attribute forced on users |
| `src/services/reports.rs` `render_template`, `render_template_text` | windmill and velvet report and communication tasks | authenticated admin (templates) with voter-influenced data | default Handlebars HTML escaping; `sanitize_html`; `url_encode` |
| `src/services/pdf.rs` `PdfRenderer::render_pdf_with_sensitivity`, `html_to_pdf` | windmill, velvet | internal service rendering admin- and voter-influenced HTML | `contains_sensitive_data` flag; private `tempdir` |
| `src/services/s3.rs` `get_upload_url`, `get_document_url`, `get_files_from_s3`, `get_files_names_bytes_from_s3`, `download_s3_file_to_string` | harvest, windmill | internal service acting for admins and voters | tenant/event/document key layout; presign expiry from env; export allowlist |
| `src/signatures/ecies_encrypt.rs` | windmill consolidation, velvet | internal service | temp files from `src/util/temp_path.rs` `generate_temp_file` |
| `src/sqlite/` (`candidate.rs` `import_candidate_sqlite` and the results tables) | windmill and velvet results database | internal service | `params!` placeholders |
| `src/plugins_wit/*/*.wit` | windmill and harvest plugin hosts | plugin code | defined here, enforced by the host |
| `src/services/probe.rs` `ProbeHandler` | Kubernetes probes | untrusted network | fixed responses; status from service-supplied checks |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| sequent-core-T1 | Information disclosure | Vote plaintext or encryption randomness leaves the voter's device with the cast ballot, or is logged by this crate. | `src/ballot.rs` `From<&AuditableBallotContest>` for `HashableBallotContest` and `TryFrom<&AuditableBallot>` for `SignedHashableBallot`, and `src/multi_ballot.rs` `TryFrom<&AuditableMultiBallot>` for `HashableMultiBallot`, keep only ciphertext and proof; `src/ballot_codec/`, `src/plaintext.rs` and `src/encrypt.rs` log only in `#[cfg(test)]` modules; `src/util/voting_screen.rs` writes only selection counts to the voter's own browser console; `DecodedBallotsInclusionPolicy` defaults to `NOT_INCLUDED`. | Mitigated |
| sequent-core-T2 | Information disclosure | Weak or predictable randomness in ballot encryption, proofs, signing keys or generated secrets. | strand `StrandRng` uses `OsRng` (the browser's `crypto.getRandomValues` in WASM); `src/services/keycloak/realm_password_policy.rs` `generate_password` (`OsRng`); `src/services/keycloak/realm.rs` `generate_client_secret` (`thread_rng`, ChaCha seeded from the OS). | Mitigated |
| sequent-core-T3 | Tampering | Malformed group elements, non-canonical scalars, trailing bytes or a re-randomized ciphertext in a submitted ballot. | strand Borsh deserialization decompresses points and requires canonical scalars; `try_from_slice` rejects trailing bytes; `src/serialization/base64.rs` uses canonical `STANDARD_NO_PAD`; `TYPES_VERSION` checks in `src/ballot.rs` and `src/multi_ballot.rs`; `src/encrypt.rs` `encrypt_plaintext_candidate` attaches a Schnorr proof of knowledge, checked by the client and by windmill at cast. | Mitigated |
| sequent-core-T4 | Tampering | A ballot is replayed, copied into another voter's cast, or counted in an election or contest it was not cast for. | `src/ballot.rs` `get_ballot_bytes_for_signing` frames `ballot_id`, `election_id` and content with length prefixes; `src/encrypt.rs` `hash_ballot` commits to the contest ciphertexts and proofs. | Partial |
| sequent-core-T5 | Tampering | The ballot tracker does not commit unambiguously to what was cast. | `src/encrypt.rs` `hash_ballot`, `hash_multi_ballot`, recomputed by windmill at cast and by ballot-verifier through `hash_auditable_ballot_js`. | Partial |
| sequent-core-T6 | Tampering | A ballot does not match the voter's ballot style (contest set, acclaimed contests, style hash). | `src/encrypt.rs` `check_contest_ids_match_style`, `votable_decoded_contests`; `src/plaintext.rs` `check_ballot_contests_match_style`; the `#[borsh(skip)]` acclaimed flag is documented in `src/ballot.rs` and pinned by `acclaimed_flag_does_not_change_contest_borsh_bytes`. Server-side checks belong to windmill (see Assumptions). | Partial |
| sequent-core-T7 | Tampering | The voter's device encrypts something other than the voter's choice (cast-as-intended). | `src/encrypt.rs` `verify_auditable_ballot_ciphertexts`, `verify_auditable_multi_ballot_ciphertext` re-encrypt from plaintext and randomness with an exact, duplicate-free contest-set match; `src/wasm/wasm.rs` `ciphertext_check_result` fails closed. End-to-end cast-as-intended also depends on the verifiers (see Assumptions). | Partial |
| sequent-core-T8 | Information disclosure | Ballots are encrypted under a key other than the election's ceremony key. | `src/ballot.rs` `PublicKeyConfig.is_demo` marks demo keys, which voting-portal shows as demo mode. End-to-end key authenticity depends on windmill, braid and the verifiers. | Not verified |
| sequent-core-T9 | Spoofing | The voter ballot signature is taken as proof of voter identity. | `src/ballot.rs` `sign_hashable_ballot_with_ephemeral_voter_signing_key` uses a fresh Ed25519 key per ballot and returns only the public key and signature. | Accepted (by design: the key is self-generated, so the signature only binds content to `ballot_id` and `election_id`; the Keycloak session authenticates the voter) |
| sequent-core-T10 | Denial of service | A decrypted plaintext with an out-of-range length byte panics the decoder and aborts the tally. | `src/ballot_codec/vec.rs` `decode_array_to_vec`. | Open on release/10.0 (fix in sequentech/step#3522) |
| sequent-core-T11 | Denial of service | Hostile decrypted plaintexts or contest configurations stop tally decoding or are counted wrongly. | `src/ballot_codec/raw_ballot.rs` `decode_from_raw_ballot` records encoding problems as `invalid_errors`; length and range checks in `src/ballot_codec/`. | Partial |
| sequent-core-T12 | Information disclosure | The ACM private key PEM and the ECIES tool commands (with the transmission password) are written to logs, and the ECIES tool runs through `sh -c`. | `src/signatures/ecies_encrypt.rs` `ecies_sign_data`, `ecies_encrypt_string`; `src/signatures/shell.rs` `run_shell_command`. | Open on release/10.0 (fix in sequentech/step#3522) |
| sequent-core-T13 | Information disclosure | Service or third-party credentials, bearer tokens, voter PII or vote content reach logs, tracing spans, `Debug` output or error strings. | Tracing spans skip selected sensitive arguments; realm attributes are logged through `redacted_attributes`. | Partial |
| sequent-core-T14 | Spoofing | Forged, expired or cross-tenant bearer tokens are accepted by backend routes. | `src/services/authorization.rs` `authorize` checks the tenant claim (or the `SUPER_ADMIN_TENANT_ID` tenant where the route allows it) and permission roles, `authorize_voter_election` and `authorize_voter_event` check area, election, event and client claims; network isolation (deployment); Datafix credentials are exchanged at Keycloak. | Not verified |
| sequent-core-T15 | Elevation of privilege | Template data or admin-written templates inject markup or script into rendered documents, emails or PDFs. | `src/services/reports.rs` `get_registry` keeps Handlebars default HTML escaping and registers templates only from strings; `sanitize_html` uses an ammonia allowlist; `url_encode` keeps a value inside one query parameter. | Partial |
| sequent-core-T16 | Information disclosure | The in-place HTML-to-PDF renderer is made to read local files or internal network endpoints, or hangs a worker. | `src/services/pdf.rs` `html_to_pdf` writes the page into a private `tempdir`; `print_to_pdf` stops after `MAX_RETRIES` attempts. | Not verified |
| sequent-core-T17 | Information disclosure | Sensitive documents are exposed through an external renderer. | `src/services/pdf.rs` `render_pdf_with_sensitivity` takes a `contains_sensitive_data` flag that can force in-place rendering. | Partial |
| sequent-core-T18 | Information disclosure | Cross-tenant object access, or misuse of presigned upload and download URLs. | `src/services/s3.rs` `get_document_key`, `get_public_document_key` namespace keys by tenant, event and document; `exportable_document_key`, `is_exportable_document` allowlist on export (tested); `src/util/aws.rs` `get_upload_expiration_secs`, `get_fetch_expiration_secs` limit URL lifetime; S3 credentials come only from env. | Partial |
| sequent-core-T19 | Elevation of privilege | An imported realm changes identity configuration beyond the intended scope. | `src/services/keycloak/realm.rs` `upsert_realm` rewrites portal client URIs from server env and forces the tenant attribute on imported users. | Partial |
| sequent-core-T20 | Elevation of privilege | A WASM plugin uses the database interfaces it is given to reach other tenants' data or credentials. | `src/plugins_wit/transaction/transaction-world.wit` gives plugins parameterized SQL on the Hasura and Keycloak databases, so a plugin is as trusted as its host; which plugins load and how they are isolated are the plugin host's responsibility (see Assumptions). | Not verified |
| sequent-core-T21 | Tampering | SQL injection into the SQLite results database, or tampered import files accepted. | `src/sqlite/` uses `params!` placeholders; `src/sqlite/utils.rs` `ensure_column` interpolates only documented literals; `src/util/integrity_check.rs` `integrity_check` compares a SHA-256 with the expected value to detect corruption. | Partial |
| sequent-core-T22 | Information disclosure | Outbound calls to Keycloak, S3, renderers or public assets are intercepted. | `Cargo.toml` builds `reqwest` with `rustls-tls` and `default-features = false`; no code in `src/` disables certificate checks; endpoint URLs, and so the scheme, come from deployment configuration. | Partial |
| sequent-core-T23 | Tampering | A vulnerable or tampered dependency or WASM build reaches the voter's browser or the backend services. | `Cargo.toml` pins `wasm-bindgen` exactly and `ed25519-dalek`, `curve25519-dalek` and `rand` to a minor version; `deny.toml` holds an advisory and licence policy; the workspace `Cargo.lock` is committed. | Partial |
| sequent-core-T24 | Tampering | A voter signature on a cast ballot does not bind the ballot to the voter. | `src/ballot.rs` `verify_ballot_signature` and `src/multi_ballot.rs` `verify_multi_ballot_signature` reject a signature that does not verify over `get_ballot_bytes_for_signing`; the cast path belongs to windmill (see Assumptions). | Not verified |
| sequent-core-T25 | Elevation of privilege | An admin reads or changes Keycloak users, groups or roles outside their tenant or election event. | Authorization by the caller (harvest, windmill). | Not verified |
| sequent-core-T26 | Information disclosure | Realm secrets are disclosed through realm export or import. | `src/services/keycloak/realm.rs` `get_realm` uses Keycloak partial export; `src/services/keycloak/realm_attributes.rs` `apply_realm_attribute_updates` ignores the redaction placeholder on update. | Not verified |

## Assumptions

- Bearer tokens are issued by Keycloak, and harvest is reachable only through Hasura or the deployment's ingress.
- The client location headers that `UserLocation` reads are set by a trusted proxy.
- windmill's cast path validates each ballot against the voter's eligibility and the event's ballot policies before storing it.
- braid's key ceremony produces the election public key that windmill publishes into ballot styles.
- ballot-verifier and voting-portal call `verify_auditable_*_ciphertext_js` before trusting decoded selections.
- voting-portal keeps the auditable ballot only in memory and only for the audit flow.
- strand, curve25519-dalek and ed25519-dalek provide correct, constant-time group arithmetic, proofs and signatures.
- Operators restrict access to container logs and use HTTPS endpoints for Keycloak, S3, the renderer and public assets.
- External document renderers run inside the deployment's trust boundary.
- harvest and windmill authorize every admin action against the tenant and election event it affects.
- Only trusted administrators can change the templates and plugins the services load.
- The Java tool at `ECIES_TOOL_PATH` (`/usr/local/bin/ecies-tool.jar`) is part of a trusted, integrity-protected image.
- The plugin hosts in windmill and harvest isolate the plugins they run.

## Review focus

1. Every decoder reachable from decrypted plaintexts (`src/ballot_codec/`, `src/plaintext.rs`), because a voter controls the encrypted bytes.
2. Ballot integrity from creation to cast: proofs, signatures and trackers (`src/encrypt.rs`, `src/ballot.rs`, `src/multi_ballot.rs`) and the windmill checks that consume them.
3. Logging and tracing hygiene for secrets, tokens and PII.
4. Authentication and authorization of backend requests (`src/services/connection.rs`, `src/services/authorization.rs`) and of Keycloak administration (`src/services/keycloak/`).
5. Authenticity of the election public key end to end (`src/ballot_style.rs`, `src/encrypt.rs`).
6. Template rendering and output escaping (`src/services/reports.rs`).
7. Isolation of the HTML-to-PDF renderer and its transports (`src/services/pdf.rs`).
8. Realm export and import (`src/services/keycloak/realm.rs`, `realm_attributes.rs`).
9. Object storage access (`src/services/s3.rs`).
10. Secret lifetime: zeroization of keys and passwords, temp files holding key material, and how the ECIES tool receives secrets after sequentech/step#3522.
11. Plugin host interfaces (`src/plugins_wit/`) and what they let a plugin reach.
12. Dependency hygiene and build provenance, including the pinned crypto crates.
