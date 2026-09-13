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

The native `default_features,keycloak` profile runs **302 passing tests, none
ignored**. Its measured source coverage is **77.24% lines** and **58.00% functions**.
LLVM regions are measured separately; actual branch coverage is not measured by
this stable native profile.

The objective is confidence in the package contracts, with high coverage as a
check on missing tests. A justified residual gap is acceptable when another test
would add little confidence. The current gaps fall into three groups:

1. **Compiled code without tests.** The largest gaps are below. Most pure helpers
   can be exercised directly; HTTP clients need controlled local responses and
   real Keycloak integration for protocol and permission behavior.
2. **Disabled or unreferenced modules.** The report inventories 47 files with no
   LLVM measurement. These include runtime code outside the selected features and other source
   requiring classification;
   absent measurements are not evidence of coverage.
3. **Measurement boundaries.** Standalone fixture and test files are excluded.
   Inline unit tests in mixed source files remain in the aggregate, so the result
   is not yet a production-only percentage. Use target-specific instrumentation to assess
   actual branches and WASM execution.

| Area | Uncovered measured lines | Tests needed |
| --- | ---: | --- |
| Keycloak services | 1,127 | Realm/user/role/permission operations; client credentials, token refresh, failed HTTP responses and retry behavior. |
| Ballot model (`ballot.rs`) | 531 | Voting-state transitions, channel-specific dates/status, contest presentation, tie resolutions and serialization boundaries. |
| Ballot codecs | 328 | Remaining malformed-input, capacity and alternate encoding paths against independent vectors. |
| Scheduled events | 111 | Tenant/event/election filtering, task names, absent/malformed payloads and scheduled-date selection. |
| Plaintext interpretation | 82 | Counting-algorithm layouts, point displays and explicit-invalid versus blank selections. |
| Request guards (`connection.rs`) | 62 | Local Rocket requests with missing/malformed credentials and valid controls; trusted versus untrusted identity inputs. |

The table covers the largest gaps, not the entire uncovered inventory. Reports
under `coverage/sequent-core/` include every measured file and the full list of
unaccounted files. Counted lines include code generated by derives where LLVM
attributes it to source; calling formatting/debug implementations solely to raise
a score does not verify an election rule.

Feature work needs separate profiles for browser/WASM exports, area trees,
reports/PDF, S3, SQLite, signature helpers, plugin execution, logging and probes.
Feature dependencies differ, so a single `--all-features` run cannot establish
coverage across native and browser targets.

Prioritize missing ballot and scheduling contracts, then local request/Keycloak
fixtures, then the remaining supported feature profiles. Keep a regression test
for every discovered defect and verify the relevant package consumers.

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
coverage percentage. The tests still execute. `summary.json` records the exclusions
and their counters separately; `llvm.raw.json` keeps the unfiltered measurements.
Both revisions in the CI comparison use this same list.

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
exceptions. Most of the current 2,803-line gap still needs meaningful tests.

## Production lint policy

Unit and integration tests may use `unwrap`, `expect`, indexing and ordinary
assertions. The additional assurance restrictions apply to production code.
`services::tally_sheet_validation` enforces the full Lightweight Assurance lint
policy in non-test builds, including checked conversions, documented contracts
and explicit failure handling.
