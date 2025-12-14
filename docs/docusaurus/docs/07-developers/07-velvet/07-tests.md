---
id: velvet_tests
title: Testing
sidebar_position: 7
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Testing

Velvet includes comprehensive test coverage to ensure correctness and reliability of the tallying system.

## Test Structure

Tests are organized in the `/packages/velvet/tests/` directory:

```
tests/
├── mod.rs
├── irv_unit_tests.rs
├── irv_integration_tests.rs
└── pipes/
    └── (pipe-specific tests)
```

## Running Tests

### All Tests

Run the complete test suite:

```bash
cargo test
```

### Specific Test

Run a single test by name:

```bash
cargo test -- test_name
```

Example:

```bash
cargo test -- cli::test_all::tests::test_hierarchical_area_aggregation
```

### Preserve Test Files

By default, tests clean up generated files. To preserve them for inspection:

```bash
export CLEANUP_FILES=false
cargo test -- test_name
```

This is useful when debugging or inspecting intermediate outputs.

## Test Categories

### Unit Tests

Test individual functions and components in isolation.

**Example**: `irv_unit_tests.rs`
- Ballot status initialization
- Vote redistribution logic
- Elimination calculations
- Tie-breaking rules
- Edge case handling

### Integration Tests

Test complete workflows end-to-end.

**Example**: `irv_integration_tests.rs`
- Full IRV tallies with multiple rounds
- Real-world election scenarios
- Complex ballot patterns
- Multi-area aggregation

### Pipe Tests

Test individual pipes independently:

**Location**: `/packages/velvet/tests/pipes/`

- Test each pipe's input/output
- Verify data transformations
- Check error handling

## Writing Tests

### Unit Test Example

```rust
#[test]
fn test_ballot_status_initialization() {
    let votes = create_test_votes();
    let contest = create_test_contest();
    
    let status = BallotsStatus::initialize_statuses(&votes, &contest);
    
    assert_eq!(status.count_valid, expected_valid);
    assert_eq!(status.count_invalid_votes.explicit, expected_invalid);
}
```

### Integration Test Example

```rust
#[test]
fn test_irv_full_tally() {
    let tally = create_test_tally();
    let algorithm = InstantRunoff::new(tally);
    
    let result = algorithm.tally().unwrap();
    
    assert_eq!(result.candidate_result[0].candidate.id, "winner-id");
    assert!(result.candidate_result[0].percentage_votes > 50.0);
}
```

## Test Fixtures

Test data and fixtures are located in:

**Location**: `/packages/velvet/src/fixtures/`

These fixtures provide:
- Sample election configurations
- Test ballot data
- Expected result sets
- Edge case scenarios

## Test Coverage

Aim to test:

- **Happy paths** - Normal operation
- **Edge cases** - Boundary conditions
- **Error handling** - Invalid inputs
- **Tie scenarios** - Tied votes and eliminations
- **Exhausted ballots** - Ballots with no remaining choices
- **Invalid ballots** - Explicit and implicit invalid votes
- **Aggregation** - Combining results from multiple areas

## Continuous Integration

Tests are automatically run as part of the CI/CD pipeline to ensure:

- New code doesn't break existing functionality
- All counting algorithms produce correct results
- Edge cases are properly handled

## Debugging Tests

### Enable Logging

To see detailed logs during tests:

```bash
RUST_LOG=debug cargo test -- --nocapture
```

### Inspect Test Output

When `CLEANUP_FILES=false` is set, you can inspect:

- Generated result files
- Intermediate pipe outputs
- Log files
- Database contents

### Run with Debugger

Use your IDE's debugger or:

```bash
rust-lldb target/debug/deps/velvet-<hash>
```

*Further documentation to be added.*
