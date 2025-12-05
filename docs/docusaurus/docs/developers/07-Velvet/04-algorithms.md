---
id: velvet_algorithms
title: Counting Algorithms
sidebar_position: 4
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Counting Algorithms

Velvet supports multiple counting algorithms to accommodate different electoral systems.

## Overview

Each counting algorithm implements the `CountingAlgorithm` trait, which defines:

```rust
pub trait CountingAlgorithm {
    fn tally(&self) -> Result<ContestResult>;
}
```

This trait-based design allows Velvet to support any electoral system by implementing the appropriate counting logic.

## Available Algorithms

### Plurality at Large

Simple majority voting where candidates with the most votes win. Commonly used for multi-seat elections where voters can select multiple candidates.

**Location**: `/packages/velvet/src/pipes/do_tally/counting_algorithm/plurality_at_large.rs`

*Documentation to be added.*

### Instant Runoff Voting (IRV)

Ranked-choice voting system where voters rank candidates in order of preference. If no candidate has a majority, the candidate with the fewest votes is eliminated and their votes are redistributed until a winner emerges.

**Location**: `/packages/velvet/src/pipes/do_tally/counting_algorithm/instant_runoff.rs`

See: [Instant Runoff Algorithm Documentation](./05-instant-runoff.md)

## Common Utilities

All algorithms share common utilities for:

- Ballot validation
- Vote counting
- Metric calculation
- Result formatting

**Location**: `/packages/velvet/src/pipes/do_tally/counting_algorithm/common.rs`

## Adding New Algorithms

To add a new counting algorithm:

1. Create a new file in `/packages/velvet/src/pipes/do_tally/counting_algorithm/`
2. Implement the `CountingAlgorithm` trait
3. Add the algorithm to `mod.rs`
4. Configure the algorithm in the election configuration

The trait-based design ensures your algorithm will integrate seamlessly with the rest of Velvet.
