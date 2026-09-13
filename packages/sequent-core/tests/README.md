<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Sequent Core Tests

From the repository's `packages` directory:

```bash
cargo test --locked -p sequent-core --features default_features,keycloak
cargo clippy --locked --no-deps --lib -p sequent-core --features default_features,keycloak
```

The native profile needs both features: the package's default feature list is
empty. Tests use synthetic local data and need no production credentials. Real
identity-provider integration and browser/WASM verification are separate scopes.

The database mapping tests also require PostgreSQL server binaries. On Debian or
Ubuntu, install `postgresql libpq-dev`; elsewhere set `PG_BIN` to the directory
containing `initdb`, `pg_ctl` and `postgres` (otherwise `pg_config --bindir` is
used). Run as an ordinary user: PostgreSQL refuses to initialize as root.
Each database test initializes its own synthetic cluster in an owner-only
temporary directory, disables TCP listening, connects through a private Unix
socket and stops the server on exit. No database URL, existing database or
production credentials are used. Missing fixture tools fail the tests rather
than silently skipping them. The Core test and coverage CI jobs install these
dependencies explicitly.

## Behaviors covered

| Test file | Contract |
| --- | --- |
| `ballot_envelope.rs` | Check the 30-byte envelope against an independent byte layout, reject invalid lengths for all 256 length bytes, and propagate errors through both contest decoders. |
| `authorization_policy.rs` | Enforce tenant isolation, required permissions, explicit super-admin opt-in, voter area/election constraints and allowed client channels. Constructed claims do not test signature verification. |
| `voting_policies.rs` | Distinguish warnings from blocked navigation for blank votes, overvotes, undervotes, ranked choices, acclaimed contests and invalid markers. |
| `ballot_style_construction.rs` | Preserve candidate/election identity, ordering, translations and encoding capacity; reject malformed presentation and annotations. |
| `ballot_signatures.rs` | Reject altered signed fields and replay into another ballot/election; reproduce ciphertext from disclosed audit randomness and preserve serialized selections. |
| `serialization_boundaries.rs` | Check independent Borsh/Base64 vectors, nested configuration errors, attribute conversion and file integrity. |
| `identity_inputs.rs` | Reject malformed claims and unrepresentable timestamps; check authentication freshness, calendar boundaries and consistent identifier replacement. |
| `tally_arithmetic_boundaries.rs` | Reject wrapped vote totals, accept valid multi-mark totals above u64, and preserve exact blank-ballot intersection bounds. |

| `codec_boundaries.rs` | Decode independent vectors; reject duplicate selections, exhausted serial numbers, invalid ranks and malformed write-in bytes. |
| `contest_policy_contracts.rs` | Check candidate-type limits, warning channels, counting algorithms, tally operations and weighted batches. |
| `keycloak_http.rs` | Inspect real HTTP paths, query parameters, payloads and rejected writes against a bounded local peer. |
| `keycloak_configuration.rs` | Check realm configuration, credential encoding, cache isolation/expiry and confidentiality of token and request diagnostics. |
| `keycloak_value_contracts.rs` | Validate password-generation limits and preserve user attributes and profile constraints. |
| `keycloak_database.rs` | Map real PostgreSQL rows into users, preserving SQL nulls and flags; reject invalid JSON objects, missing columns and incompatible SQL types. |
| `model_contracts.rs` | Validate persisted nested configuration and ceremony, tally, result and event-policy defaults. |
| `plaintext_display.rs` | Check voting layouts, displayed points and invalid-versus-blank selections. |
| `policy_wire_format.rs` | Macro-generated contract tests pin explicit JSON policy names and Borsh discriminants, reject incomplete/unknown inputs and propagate stream failures. |
| `ballot_wire_streams.rs` | Pin independent byte layouts for small records; reject every truncated prefix and propagate sink failures through nested ballot, presentation and tally-resolution records. |
| `presentation_contracts.rs` | Check translated-name fallbacks, languages and presentation policies. |
| `request_guards.rs` | Dispatch local Rocket requests with valid, absent and malformed headers; claims parsing does not verify signatures. |
| `scheduled_dates.rs` | Filter by tenant, event, election and task; preserve missing dates and reject malformed payloads. |
| `utility_contracts.rs` | Check time, authentication URLs, numeric ordering, external configuration and file-integrity errors. |
| `voting_state.rs` | Check channel transitions, first-transition dates and early-voting closure. |

The mixed-radix unit test uses independently specified vectors for legacy and
expanded-capacity encoding, with decline disabled and enabled. Unordered IDs,
unset interior slots, invalid empty contests and trailing padding cannot shift
the expected slot layout.

## Decoder API

`ballot_codec::decode_array_to_vec` returns `Result<Vec<u8>, String>`. Propagate
errors with `?` in fallible callers, or assert success explicitly in tests.
Single-contest and multi-contest decoders propagate invalid envelope lengths.
Payload errors describe the length without including plaintext contents.

## Coverage and remaining work

The native `default_features,keycloak` profile runs **484 passing tests, none
ignored**. Source commit `11dec93c4484e6f7876c2045521a0b554a0eeb99` measures
**11,921/12,315 lines (96.80%)**, **1,398/1,465 functions (95.43%)** and
**15,194/15,885 LLVM regions (95.65%)** using Rust 1.96.0 and cargo-llvm-cov 0.9.1.
Production Clippy and workspace formatting pass (existing warnings remain).
Actual branch coverage is not measured by this stable native profile.

The suite uses bounded local HTTP peers for Keycloak and Rocket dispatch for
request guards. It verifies request payloads, authentication failures, token-cache
isolation and expiry without contacting a production identity provider. These
checks complement, but do not replace, integration against a running Keycloak.

The report has **394 uncovered measured lines** and **67 uncovered functions**.
The new cases exercise generated stream contracts, PostgreSQL row mapping,
malformed audit/hash payloads, permission-label deduplication, expired tokens,
rejected realm/user/permission writes and invalid user locations. Oversized
mixed-radix payloads and group updates without an id reproduced panics before
their fixes; zero radices are also rejected.
Continue with the remaining realizable Keycloak transport/refresh failures,
preferential ballot validation and service integration. These remain obligations,
not exceptions justified by the aggregate percentage.

The report inventories **47 files without an LLVM measurement**, classified
below. Their supported configurations still need separate measurements; the
runner continues to list these files as unaccounted rather than treating a prose
classification as coverage. The native line improvement target is met, but the
strict overall target remains open. A single native `--all-features` run cannot
establish browser coverage.

Standalone fixtures and test files are excluded. Inline tests in mixed source
files remain counted, so this is **not a production-only percentage**. Do not
exercise test-only tree printing, assertion-failure formatting or routine derives
solely to raise the score. Keep runtime validation and cryptography measured.

Measure from the repository root:

```bash
python3 scripts/coverage/run.py sequent-core --baseline
```

The improvement target is at least 95%, approaching complete coverage where tests
add confidence. Document low-value residual gaps instead of forcing 100%.
The CI policy is **no decrease against the PR base**, separately for each measured
metric. A passing comparison does not mean the improvement target is complete.
Track the remaining work in [Meta #13292](https://github.com/sequentech/meta/issues/13292).

## Unmeasured source inventory

Paths below are relative to `src/`. This classification covers all 47 files in
the measured revision and does not add exclusions or alter report counters.
Module-only files in disabled directories are included with their owning feature.

| Files | Native profile status and next evidence needed |
| --- | --- |
| `error.rs`, `types/error.rs` | Native error declarations expanded by `quick_error!`; no separate LLVM source entry. Keep caller error assertions; do not manufacture errors solely to exercise generated formatting. |
| `types/permissions.rs`, `types/tally_sheet_import.rs` | Native enums/structs and derives without handwritten method bodies. Permission names and import wire formats remain consumer contracts; absence of an LLVM entry is not proof that every contract is tested. |
| `util/console_log.rs` | Macro definitions expanded at call sites; WASM console behavior needs the browser profile. |
| `services/area_tree.rs` | Disabled `areas` feature; exercise hierarchy, missing parents and cycles separately. |
| `services/pdf.rs`, `services/reports.rs`, `temp_path.rs`, `types/templates.rs`, `util/path.rs` | Disabled `reports` feature; test rendering, browser failure/cleanup, templates and temporary paths with synthetic inputs. Both Chromium `--single-process` and `--no-zygote` flags are retained. |
| `services/s3.rs` | Disabled `s3` feature; test uploads/downloads and rejected operations with a local service. |
| `util/aws.rs`, `util/temp_path.rs` | Enabled by `reports` or `s3`; require separate credential-free service and filesystem tests. |
| `services/probe.rs`, `util/retry.rs` | Probe is disabled; retry is enabled by `probe` or `reports`. Test health responses, retries and exhaustion separately. |
| `util/init_log.rs` | Disabled `log` feature; test subscriber initialization and diagnostics in isolated processes. |
| `signatures/ecies_encrypt.rs`, `signatures/shell.rs`, `signatures/mod.rs` | Disabled `signatures` feature; these helpers require separate encryption/signing and subprocess failure checks. Core ballot-signature tests do not certify this feature. |
| `plugins_wit/lib.rs`, `plugins_wit/mod.rs` | Disabled `plugins_wit` feature; validate component loading, host boundaries and trapped execution separately. |
| `sqlite/area.rs`, `sqlite/area_contest.rs`, `sqlite/candidate.rs`, `sqlite/contests.rs`, `sqlite/election.rs`, `sqlite/election_event.rs`, `sqlite/results_area_contest.rs`, `sqlite/results_area_contest_candidate.rs`, `sqlite/results_contest.rs`, `sqlite/results_contest_candidate.rs`, `sqlite/results_election.rs`, `sqlite/results_election_area.rs`, `sqlite/results_event.rs`, `sqlite/tally_session_resolution.rs`, `sqlite/utils.rs`, `sqlite/mod.rs` | Disabled `sqlite` feature (16 files). Use temporary databases for mapping, constraints, malformed JSON and persistence tests; the PostgreSQL User mapper tests do not cover these modules. |
| `wasm/templates.rs`, `wasm/wasm_hasura_types.rs`, `wasm/wasm_interpret_plaintext.rs`, `wasm/wasm_keycloak.rs`, `wasm/wasm_permissions.rs`, `wasm/wasm_plaintext.rs`, `wasm/mod.rs` | Disabled `wasm` feature; exercise exported APIs in a browser and account for them independently of native counters. |
| `wasm/areas.rs`, `wasm/wasm.rs` | Additionally gated by `wasmtest`; browser test sources need separate accounting. |

## Coverage exclusions

The native profile lists exact filenames and reasons in
[`scripts/coverage/profiles.toml`](../../../scripts/coverage/profiles.toml).
The source files also have a comment pointing to that policy.

| Excluded file | Reason |
| --- | --- |
| `src/fixtures/ballot_codec.rs` | Synthetic ballots, expected vectors and test builders; no ballot validation or cryptography implementations. |
| `src/fixtures/encrypt.rs` | Sample election data compiled only under `cfg(test)`. |
| `src/election_config/validate_tests.rs` | Standalone unit tests; the validator they exercise stays in the report. |

These files are omitted from `llvm.json`, HTML, LCOV, missing-line output and the
coverage percentage. The tests still execute. Excluded code contributes to neither
the covered count nor the total count, for lines, functions or regions.
`summary.json` lists excluded paths and reasons without retaining their counters. Both CI revisions use the same
list. No unfiltered coverage report is generated.

To exclude another reviewed support file, add its exact package-relative path
and reason under `[profiles.sequent-core.excluded_files]`. Wildcards, missing
files, empty reasons and overlaps with `scope_exceptions` fail validation.
Declaration-only files belong in `scope_exceptions`; those entries cannot hide
measured executable code.

LLVM's JSON filename filter removes file records and counters but leaves function
records behind. The runner makes the function list follow the exported file
inventory, including Cargo's automatic test/dependency exclusions, before
publishing `llvm.json`, without changing counters. An expansion mixing
excluded and included files fails validation rather than hiding production code.
Regression tests check both the exact retained records and unchanged counters.

The infallible-error rationales below do not exclude their
containing production files. The stable runner filters whole files, so mixed
runtime/test modules remain measured until their boundaries can be separated.

## Generated serialization functions

Generated serialization is a wire contract, not a diminishing-return exception.
With Rust 1.96.0, cargo-llvm-cov 0.9.1 and Borsh 1.5.7, a minimal struct's successful
encode/decode assertions left both derived function counters at zero. Adding
truncated-input and failing-writer controls registered both functions. Each
function had a single mapped region at the derive invocation; its reported count
therefore must not be read as a count of all successful calls.

`policy_contract!` and `record_contract!` generate assertions from explicit test
cases, not from production enum iteration or serializers. Small records pin
literal expected bytes. The shared stream helper rejects every proper prefix,
preserves an independently injected `PermissionDenied` error at every byte, checks
a successful full-capacity writer and rejects trailing data. Larger synthetic
ballots use the same stream-failure properties with valid ciphertexts and proofs.
These are error-propagation tests; a large-record round trip alone is not treated
as independent evidence of its byte layout.

This work raised covered functions from 1,197 to 1,375 without changing the 1,465
denominator. Subsequent tests address handwritten behavior separately. No Borsh
function remains wholly uncovered in the measured profile. Generated functions
stay in counters and all exports; no `coverage(off)` attributes or new exclusions
were added. Inline test diagnostics and unmeasured feature profiles remain visible.

The remaining **67 unexecuted functions** are located as follows (paths relative
to `src/`). Counts include closures, not just named public APIs:

| Source | Unexecuted functions | Review direction |
| --- | ---: | --- |
| `ballot_codec/multi_ballot.rs` | 24 | 18 inline test diagnostics; five numeric-conversion errors and one lookup guard. Preserve the 64-bit conversion and prior-validation rationale below. |
| `ballot.rs`, `multi_ballot.rs` | 18 | Signing/serialization error closures plus the manual `EInitializeReportPolicy::default`. Generated Borsh implementations are covered; review the concrete backend/error edge, not the derive name. |
| `services/keycloak/admin_client.rs` | 7 | Token-conversion/lock errors plus the still-useful interrupted HTTP body-read case in `get_credentials_inner`. |
| `ballot_codec/raw_ballot.rs` | 4 | Two inline assertion diagnostics, a prior-validated candidate lookup and direct raw-choice conversion overflow. The latter remains a useful rejected-input test. |
| `encrypt.rs` | 4 | Prior-validated contest lookups, ballot-style serialization and `encrypt_multi_ballot`'s encoding-error propagation. The last edge remains useful to test directly. |
| `plaintext.rs` | 3 | Lookups after immutable contest-set validation and a repeated deserialization of identical bytes. |
| `ballot_codec/contest_context.rs` | 1 | Fallback text for a configuration error without a message; both current checker errors always supply a message. |
| `services/keycloak/realm_password_policy.rs` | 1 | UTF-8 conversion failure after constructing a password exclusively from ASCII character sets. |
| `services/keycloak/user.rs` | 1 | Non-hierarchical URL mutation after successful HTTP authentication against the same configured URL. |
| `services/keycloak/realm.rs` | 1 | Token-supplier failure before realm export; distinguish this from tested HTTP rejection and transport failures. |
| `util/voting_screen.rs` | 1 | `get_decoded_contest_plurality`, a fixture builder not used by this profile; do not call it merely for coverage. |
| `election_config/report.rs` | 1 | Inline assertion diagnostic. |
| `main.rs` | 1 | Empty executable entry point. |

This inventory is not an exclusion list or a claim that all remaining behavior is
infeasible. The named reachable cases and separately measured configurations stay
open in Meta #13292. Routine generated `Debug`/`Clone` code does not explain the
current function gap.

## Where additional coverage adds little value

These are specific reasons to accept a residual gap, not exemptions for whole
subsystems. Keep the explanation beside the coverage evidence. No percentage
should be raised merely by exercising unrelated implementation details.

| Code | Why a dedicated coverage test adds little | Treatment |
| --- | --- | --- |
| [`fixtures/encrypt.rs`](../src/fixtures/encrypt.rs): `get_encrypt_decoded_test_fixture` and `default_voting_portal_fixture` | Compiled only under `cfg(test)` in `fixtures/mod.rs`; their only call sites in Step are inside a commented-out test. The code supplies sample data rather than deployed election behavior. | Excluded through `excluded_files` in the native profile. Retain useful fixtures with contract checks, or remove unused ones as cleanup; do not call them just to increase a score. |
| [`ballot_codec/mod.rs`](../src/ballot_codec/mod.rs), [`serialization/mod.rs`](../src/serialization/mod.rs), and import-only [`ballot_verifier.rs`](../src/ballot_verifier.rs) | These files contain declarations, re-exports, a marker trait or imports without executable bodies. There is no runtime outcome for a unit test to exercise. | Listed as non-executable source in `scope_exceptions`; compilation and consumer tests check the interfaces. An exception fails if LLVM measures executable code in that file. |
| The serialization-error edge in [`generate_voting_period_dates`](../src/types/scheduled_event.rs) | `serde_json::to_value` receives `ManageElectionDatePayload`, a derived struct containing only `Option<String>`. This value has no recoverable serialization-error case. | Do not alter production design or fabricate a failing serializer solely to hit this edge. Test `Some`/`None`, filtering and resulting dates; revisit the rationale if the payload gains fallible fields. |
| `ballot_codec/multi_ballot.rs`: inline `TreeItem`/`Display` implementations and assertion-failure branches | These render test-only trees or explain a failed assertion. Executing them does not verify an election rule. | Keep their mixed source file measured; accept the residual diagnostic lines. |
| `encrypt.rs`: contest-not-found closures in `encrypt_decoded_contest`; `plaintext.rs`: matching closures and the second multi-contest deserialization | Earlier immutable contest-set validation guarantees the lookup, and the same immutable payload was already deserialized successfully. | Exercise missing/duplicate/unknown contest rejection at the reachable boundary; do not bypass validation to hit redundant guards. |
| `ballot_codec/multi_ballot.rs`: candidate-position `usize` to `u64` and remainder `BigUint` to `u64` error closures | On the measured 64-bit target, positions fit `u64`; a remainder below a positive `u64` radix also fits `u64`. | Retain validation of oversized ballot values and zero radices, with no fabricated numeric-conversion failures. |
| `services/keycloak/admin_client.rs`: JSON serialization failures in the two token conversions; poisoned cache locks | Token fields are strings, integers and options with derived serialization. Lock poisoning would require an unrelated panic inside a private critical section. | Preserve real denied/malformed-token and expiry tests. Revisit if fallible fields or critical-section behavior changes. |
| `util/integrity_check.rs`: SHA-256 computation error arm | The selected `strand::hashing::rustcrypto::hash_sha256` implementation returns `Ok` for every byte slice; it has no recoverable backend failure. | Keep file-open/read and mismatched-hash checks; do not inject a failing cryptographic backend just for coverage. |


HTTP failures, token expiry, permission rejection, malformed ballots and arithmetic
boundaries remain valuable tests even when difficult to set up. Disabled native
features and WASM are separate coverage obligations, not diminishing-return
exceptions. Review the remaining measured gaps against concrete behavior before
closing the package coverage task.

## Production lint policy

Unit and integration tests may use `unwrap`, `expect`, indexing and ordinary
assertions. The additional assurance restrictions apply to production code.
`services::tally_sheet_validation` enforces the full Lightweight Assurance lint
policy in non-test builds, including checked conversions, documented contracts
and explicit failure handling.
