<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Core boundary tests

From the repository's `packages` directory:

```bash
cargo test --locked -p sequent-core --features default_features,keycloak
```

The native profile needs both features: the package's default feature list is
empty. These tests use synthetic local data and require no running identity
provider or production credentials. They do not replace JWT verification tests,
real service integration tests or the browser/WASM test suite.

- `ballot_envelope.rs` checks the 30-byte ballot envelope against a manually
  specified byte layout, tests all 256 length bytes, and checks error propagation
  through the single-contest and multi-contest decoders. Before the fix, a length
  byte of 30 panicked instead of returning an error.
- `authorization_policy.rs` checks tenant isolation, every required permission,
  the explicit super-admin opt-in, voter area/election constraints and the allowed
  client-to-channel mapping. Removing the permission checks makes three of these
  tests fail. Claims are constructed directly; their signatures are outside this
  test boundary.
- The restored `test_mixed_radix_encode` unit test uses written-out vectors for
  legacy and expanded-capacity encoding, with decline disabled and enabled. Its
  fixtures include unordered IDs, an unset interior slot, an explicitly invalid
  empty contest and trailing padding. It no longer depends on random draws or
  searching past zero slots to guess the next contest's offset.

New integration assertions live outside `src`, so they do not inflate package
source coverage. Existing inline tests and fixture helpers still contribute to
the native aggregate and must be considered when interpreting it.

## Decoder API change

`ballot_codec::decode_array_to_vec` now returns `Result<Vec<u8>, String>`. Propagate
the error with `?` in fallible callers, or assert success explicitly in tests.
The single-contest and multi-contest entry points already return `Result` and now
propagate invalid envelope lengths. Valid ballot bytes are unchanged. Oversized
payload errors describe the length without including plaintext contents.

The package-wide 95% coverage target remains tracked in
[Meta #13292](https://github.com/sequentech/meta/issues/13292). Passing these boundary
tests alone does not close that target or establish deployment-level security.

## Extended boundary checks

The additional integration test files exercise production entry points with
synthetic data and explicit expected outcomes:

- `voting_policies.rs`: distinguish warnings from blocked navigation; check blank,
  overvote, undervote and ranked-choice rules, missing decoded state, acclaimed
  contests, and explicit-invalid markers.
- `ballot_style_construction.rs`: preserve election and candidate identity,
  deterministic ordering, translations, demo-key markings, and election-wide
  encoding capacity; reject malformed presentation and annotation fields.
- `ballot_signatures.rs`: reject replay into another ballot or election and changes
  to signed fields; reproduce ciphertext from disclosed audit randomness and
  preserve decoded selections through the public serialization boundaries.
- `serialization_boundaries.rs`: independently specified Borsh/Base64 bytes,
  nested configuration error paths, attribute conversion and file integrity.
- `identity_inputs.rs`: malformed claims, authentication freshness, extreme
  timestamps, calendar boundaries and identifier replacement consistency.

The timestamp regression failed with an integer-overflow panic before the fix.
Timestamp parsing now rejects unrepresentable dates without multiplying seconds
into milliseconds. Claims tests do not replace identity-provider signature or
service integration tests.

- `tally_arithmetic_boundaries.rs`: rejects wrapped vote totals, accepts valid
  multi-mark sums above a single u64 counter and verifies blank-ballot
  intersection bounds near the numeric limit. Five cases panicked before the
  shared validator widened its intermediate arithmetic; the public count types
  and validation codes remain unchanged.

## Production lint policy

Unit and integration tests may use `unwrap`, `expect`, indexing and ordinary
assertions. The additional assurance policy applies to production code.

From `packages/`, run:

```sh
cargo clippy --locked --no-deps --lib -p sequent-core --features default_features,keycloak
```

The first production module to enforce the full Lightweight Assurance policy is
`services::tally_sheet_validation`. Its non-test build rejects unchecked panic
shortcuts, undocumented contracts and the other agreed lints. Existing lint debt
in other Core modules remains tracked under Meta #11566. The two ballot encoder
helpers keep their existing implementation.
