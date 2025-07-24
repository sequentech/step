<!--
SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
# Integration Tests

This directory contains integration tests for the sequent-core crate.

## Test Organization

- `encrypt.rs` - Integration tests for the encrypt module, including:
  - `test_multi_contest_reencoding_with_explicit_invalid` - Tests multi-contest reencoding with explicit invalid candidates

## Running Tests

To run all integration tests:
```bash
cargo test --features keycloak
```

To run a specific integration test:
```bash
cargo test --features keycloak test_multi_contest_reencoding_with_explicit_invalid
```

## Unit Tests vs Integration Tests

- **Unit tests** remain in the source files within `#[cfg(test)]` modules for testing private functions and implementation details
- **Integration tests** in this directory test the public API and end-to-end functionality
