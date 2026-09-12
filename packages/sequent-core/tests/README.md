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
