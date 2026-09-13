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
| `policy_wire_format.rs` | Pin JSON policy names and Borsh discriminants used in published ballot styles. |
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

The native `default_features,keycloak` profile runs **391 passing tests, none
ignored**. Its measured source coverage is **94.01% lines** and **79.86% functions**.
LLVM regions are measured separately; actual branch coverage is not measured by
this stable native profile.

The suite uses bounded local HTTP peers for Keycloak and Rocket dispatch for
request guards. It verifies request payloads, authentication failures, token-cache
isolation and expiry without contacting a production identity provider. These
checks complement, but do not replace, integration against a running Keycloak.

The report has **737 uncovered measured lines**. Continue with realizable
failure paths in the remaining Keycloak operations, malformed ballot and audit
inputs, and any election rules without an independent assertion. Database row
mapping needs a local PostgreSQL fixture. Generated serialization errors and
inline test diagnostics require individual review before proposing more tests.

The report inventories **47 files without an LLVM measurement**. Supported WASM,
area-tree, reports/PDF, S3, SQLite, signature-helper, plugin, logging and probe
profiles need separate tests and source classification. Feature dependencies
vary; a single native `--all-features` run cannot establish browser coverage.

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

The generated-code and infallible-error rationales below do not exclude their
containing production files. The stable runner filters whole files, so mixed
runtime/test modules remain measured until their boundaries can be separated.

## Where additional coverage adds little value

These are specific reasons to accept a residual gap, not exemptions for whole
subsystems. Keep the explanation beside the coverage evidence. No percentage
should be raised merely by exercising unrelated implementation details.

| Code | Why a dedicated coverage test adds little | Treatment |
| --- | --- | --- |
| [`fixtures/encrypt.rs`](../src/fixtures/encrypt.rs): `get_encrypt_decoded_test_fixture` and `default_voting_portal_fixture` | Compiled only under `cfg(test)` in `fixtures/mod.rs`; their only call sites in Step are inside a commented-out test. The code supplies sample data rather than deployed election behavior. | Excluded through `excluded_files` in the native profile. Retain useful fixtures with contract checks, or remove unused ones as cleanup; do not call them just to increase a score. |
| Generated `Debug` and `Clone` implementations on ballot data types in [`ballot.rs`](../src/ballot.rs) | Testing every generated field copy or debug rendering mostly retests Rust derives. | Exercise them through real scenarios. Test explicit privacy/redaction and copy-isolation requirements if present. Serialization, permission strings and signed bytes remain important contracts. |
| [`ballot_codec/mod.rs`](../src/ballot_codec/mod.rs), [`serialization/mod.rs`](../src/serialization/mod.rs), and import-only [`ballot_verifier.rs`](../src/ballot_verifier.rs) | These files contain declarations, re-exports, a marker trait or imports without executable bodies. There is no runtime outcome for a unit test to exercise. | Listed as non-executable source in `scope_exceptions`; compilation and consumer tests check the interfaces. An exception fails if LLVM measures executable code in that file. |
| The serialization-error edge in [`generate_voting_period_dates`](../src/types/scheduled_event.rs) | `serde_json::to_value` receives `ManageElectionDatePayload`, a derived struct containing only `Option<String>`. This value has no recoverable serialization-error case. | Do not alter production design or fabricate a failing serializer solely to hit this edge. Test `Some`/`None`, filtering and resulting dates; revisit the rationale if the payload gains fallible fields. |

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
