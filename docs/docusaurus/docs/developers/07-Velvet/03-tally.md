---
id: velvet_tally
title: Tally Engine
sidebar_position: 3
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Tally Engine

The tally engine is the core component responsible for counting votes and determining election results.

## Overview

The tally engine processes decoded ballots using a specified counting algorithm to produce:

- Vote totals for each candidate
- Percentage of votes received
- Valid, invalid, and blank vote counts
- Extended metrics for analysis
- Winner determination (if applicable)

## Architecture

The tally engine is implemented in the `do_tally` pipe and consists of:

- **Tally struct** - Holds ballot data, contest configuration, and census information
- **Counting algorithms** - Pluggable algorithms that implement different electoral systems
- **Result aggregation** - Combines results from multiple areas or districts
- **Metrics tracking** - Records detailed statistics about the voting process

## Ballot Processing

Ballots go through several stages:

1. **Validation** - Check if ballots are valid, invalid, or blank
2. **Counting** - Apply the counting algorithm based on the electoral system
3. **Aggregation** - Combine results from multiple voting areas
4. **Metric calculation** - Compute percentages and statistics

## Invalid Votes

The tally engine handles different types of invalid votes:

- **Explicit invalid** - Voter intentionally marked ballot as invalid
- **Implicit invalid** - Ballot is invalid due to errors (e.g., overvoting)
- **Blank votes** - Ballot has no selections

Invalid vote handling can be configured per election.

## Extended Metrics

The tally engine tracks detailed metrics including:

- Total ballots cast
- Number of valid ballots
- Number of invalid ballots (explicit and implicit)
- Number of blank ballots
- Voter participation rates
- Per-candidate vote distributions

## Result Structure

Tally results include:

```rust
ContestResult {
    contest: Contest,
    census: u64,
    auditable_votes: u64,
    total_votes: u64,
    total_valid_votes: u64,
    total_invalid_votes: u64,
    total_blank_votes: u64,
    candidate_result: Vec<CandidateResult>,
    extended_metrics: ExtendedMetricsContest,
    // ... percentage fields
}
```

Each candidate result contains:

```rust
CandidateResult {
    candidate: Candidate,
    total_count: u64,
    percentage_votes: f64,
}
```

## Location

Tally engine implementation: `/packages/velvet/src/pipes/do_tally/`

*Further documentation to be added.*
