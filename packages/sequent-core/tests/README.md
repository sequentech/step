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
empty. Tests use synthetic local data and need no production credentials. The focused profile exercises ballot and identity adapters. Use the enabled
native profile below for SQLite, report and other application features.

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
| `ballot_signatures.rs` | Reject altered signed fields, either incomplete key/signature pair and replay into another ballot/election; preserve valid signed and fully unsigned controls, reproduce ciphertext from disclosed audit randomness and preserve serialized selections. |
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

## Enabled native features

From the repository root:

```bash
python3 scripts/coverage/run.py sequent-core-native --baseline --offline
```

This profile enables `areas`, `sqlite`, `reports`, `reports_sync`, `s3`, `probe`,
`log`, `signatures` and `plugins_wit` alongside `default_features,keycloak`.
It has its own CI comparison against the actual PR base; its counters are not
combined with the focused profile. SQLite uses bundled SQLite and temporary or
in-memory databases. Report rendering uses the pinned local browser installed by
`.github/actions/setup-test-browser`; keep its required process arguments.

For a focused development loop, from `packages/`:

```bash
cargo test --locked -p sequent-core --test sqlite_feature_boundaries \
  --features default_features,keycloak,sqlite
cargo test --tests --locked -p sequent-core \
  --features default_features,keycloak,areas,reports,reports_sync,s3,probe,log,signatures,sqlite,plugins_wit
```

`--tests` selects library and integration tests without building the PDF example,
which additionally requires choosing a renderer feature such as `lambda_inplace`.

| Test file | Contract |
| --- | --- |
| `sqlite_feature_boundaries.rs` | Literal SQLite cell expectations for entity and result mappings, null versus zero, Unicode metadata, CSV rejection, transaction rollback, translation overrides and document-update scope keys. |
| `s3_feature_boundaries.rs` | Complete downloads preserve literal payloads; interrupted streams fail for both memory and temporary-file APIs against a local HTTP peer. |
| `native_feature_boundaries.rs` | Area hierarchy completeness and cycle rejection, independent live/ready responses, bounded retries, template escaping/failures and temporary-file cleanup. |

SQLite tests read the database directly rather than using a second production
mapper as the oracle. Distinct counts and IDs expose column swaps; valid rows
remain controls for constraint, parser and scope failures. Dropping or rolling
back a transaction must preserve the caller's export boundary.

## Coverage interpretation

The profiles count generated serialization and existing inline test code. Test
files under `tests/` do not contribute to the package's `src` totals. Exact
fixture exclusions are configured in `scripts/coverage/profiles.toml` and applied
to all exports. A feature absent from one profile is not an exclusion rationale:
run its enabled profile and inspect its own source inventory.

For residual paths, inspect the specific contract. For example, a conversion from
`usize` to `u64` cannot fail on a 64-bit target, a SHA-256 implementation returning
`Ok` for every byte slice has no injectable backend error, and a second lookup
in an immutable, already-validated collection cannot independently lose its key.
Do not rewrite those interfaces or exercise test-only diagnostics just for a
counter. Revise that reasoning if the target, backend or input contract changes.

LLVM regions are not branch coverage. Browser exports run on a WASM target;
compiling a native feature profile does not verify JavaScript conversions or the
browser boundary.

## Browser tests

Install the `wasm32-unknown-unknown` target for the pinned Rust toolchain, Clang,
`wasm-bindgen-test-runner` 0.2.104 and a ChromeDriver matching your Chrome version.
The pinned downloads and checksums are in `core-native-features.yml`. Put both
executables on `PATH` and set the runner:

```bash
export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner
cd packages
cargo test --locked --target wasm32-unknown-unknown -p sequent-core \
  --features wasmtest,default_features --test mod
```

For a custom browser binary, set `WASM_BINDGEN_TEST_WEBDRIVER_JSON` to a JSON file
with `goog:chromeOptions.binary` and the required Chrome arguments (CI includes
`--single-process` and `--no-zygote`). The tests exercise real JavaScript values:
a current signed ballot succeeds, content changes and replay fail, incomplete
signature pairs are rejected, and unsigned ballots return false. An obsolete
ballot fixture verifies version rejection, not signature verification.
