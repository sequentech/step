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

## Configurable tally operations

The tally engine can be configured to perform different operations at **contest** and **area** level. This is controlled by the `tally_operation` setting, which determines whether ballots are processed in detail or results are just aggregated.

### Available operations

The `tally_operation` value must be one of the variants of the `TallyOperation` enum:

- **`process-ballots-all`** (`TallyOperation::ProcessBallotsAll`)
  - Processes all ballots in the scope (contest or area).
  - Produces full candidate results and participation statistics.
- **`aggregate-results`** (`TallyOperation::AggregateResults`)
  - Does **not** re-process ballots.
  - Aggregates candidate results that were already computed in lower-level areas.
- **`skip-candidate-results`** (`TallyOperation::SkipCandidateResults`)
  - Processes ballots only to compute participation statistics.
  - Does **not** generate candidate-level results in that scope.

### Contest-level configuration

At contest level, the tally operation is read from the contest `annotations` JSON as the `tally_operation` key:

- **Location**: `contest.annotations["tally_operation"]`
- **Type**: string
- **Allowed values**: `"process-ballots-all"`, `"aggregate-results"`, `"skip-candidate-results"`

If `tally_operation` is **not present**, empty, or contains an unknown value, the engine falls back to a default that depends on the counting algorithm (`CountingAlgType`):

- For **preferential (ranked-choice)** algorithms (`instant-runoff`, `borda`, `borda-nauru`, `borda-mas-madrid`, `pairwise-beta`, `desborda`, `desborda2`, `desborda3`):
  - **Default contest operation**: `process-ballots-all`
- For **non-preferential** algorithms (e.g. `plurality-at-large`, `cumulative`):
  - **Default contest operation**: `aggregate-results`

This behavior is implemented by `get_contest_tally_operation`, which parses `tally_operation` and falls back to `CountingAlgType::get_default_tally_operation_for_contest()` when needed.

### Area-level configuration

At area level, the tally operation is configured via the `area_annotations` associated with each `BallotStyle` and the area identifier:

- **Source**: the `BallotStyle` entry whose `area_id` matches the area being tallied.
- **Location**: `ballot_style.area_annotations.tally_operation` (internally accessed via `get_tally_operation()`).
- **Type**: string
- **Allowed values**: same as contest level – `"process-ballots-all"`, `"aggregate-results"`, `"skip-candidate-results"`.

If no matching `BallotStyle` is found for an area, or if `tally_operation` is missing/invalid in `area_annotations`, the engine uses a default that also depends on the counting algorithm:

- For **preferential (ranked-choice)** algorithms:
  - **Default area operation**: `skip-candidate-results`
- For **non-preferential** algorithms:
  - **Default area operation**: `process-ballots-all`

This behavior is implemented by `get_area_tally_operation`, which selects the corresponding `BallotStyle`, queries its `area_annotations`, and falls back to `CountingAlgType::get_default_tally_operation_for_area()`.

### Summary of defaults

| Counting algorithm type     | Contest default        | Area default              |
| --------------------------- | ---------------------- | ------------------------- |
| Preferential (ranked-choice)| `process-ballots-all`  | `skip-candidate-results`  |
| Non-preferential            | `aggregate-results`    | `process-ballots-all`     |

When in doubt or when configuration is missing, the tally engine will always choose a safe default based on the counting algorithm, so tallying can proceed even without explicit `tally_operation` settings.

## Location

Tally engine implementation: `/packages/velvet/src/pipes/do_tally/`

*Further documentation to be added.*
