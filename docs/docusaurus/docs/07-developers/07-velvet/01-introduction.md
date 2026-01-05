---
id: velvet_introduction
title: Velvet Introduction
sidebar_position: 1
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Velvet

**Velvet** is the tallying and reporting engine for the Sequent Tech voting platform. The name comes from the type of woven tufted fabric, implying smoothness and luxury in processing election results.

## Overview

Velvet is a Rust cargo crate that provides both a library and a binary executable for processing election results. It handles the entire post-voting workflow including:

- Decoding encrypted ballots
- Executing tally algorithms
- Generating reports and receipts
- Producing verifiable results

While Velvet includes a CLI tool for testing and development, its primary use is as a library integrated into the platform's backend services. The main functionality is triggered from the **Admin Portal** when an election administrator executes a tally.

## Architecture

Velvet is organized around several key components:

### Pipes

Pipes are the core processing units in Velvet. Each pipe performs a specific task in the tallying workflow:

- **decode_ballots** - Decodes encrypted ballots into plaintext votes
- **do_tally** - Executes the counting algorithm on decoded ballots
- **generate_db** - Creates a SQLite database with tally results
- **generate_reports** - Produces PDF and CSV reports
- **mark_winners** - Determines winning candidates based on tally results
- **vote_receipts** - Generates individual vote receipts for voters

Pipes can be chained together in a processing pipeline defined by configuration.

### Tally Engine

The tally engine (`do_tally`) is responsible for executing counting algorithms on ballots. It supports multiple electoral systems through a pluggable algorithm architecture.

### Counting Algorithms

Velvet supports various counting algorithms for different electoral systems:

- **Plurality at Large** - Simple majority voting where candidates with the most votes win
- **Instant Runoff Voting (IRV)** - Ranked-choice voting with elimination rounds

Each algorithm implements the `CountingAlgorithm` trait, making it easy to add new electoral systems.

### CLI

The command-line interface provides a way to run Velvet pipelines directly:

```bash
velvet run {stage} {optional-pipe} \
  --config ./path/to/velvet-config.json \
  --input-dir ./path/to/input-dir \
  --output-dir ./path/to/output-dir
```

The CLI is primarily used for development, testing, and debugging. In production, Velvet is invoked programmatically from the backend services.

### Tests

Velvet includes comprehensive test coverage:

- **Unit tests** - Test individual functions and components (e.g., `irv_unit_tests.rs`)
- **Integration tests** - Test complete tallying workflows (e.g., `irv_integration_tests.rs`)
- **Pipe tests** - Test individual pipes in isolation

Tests can be run with:

```bash
cargo test
```

### Benchmarks

Performance benchmarks are included to measure and optimize critical operations like PDF generation. Benchmarks use the Criterion framework and can be run with:

```bash
cargo bench
```

## Workflow

A typical Velvet execution flow:

1. **Input** - Election configuration and encrypted ballots are provided
2. **Decode** - Ballots are decrypted and decoded
3. **Tally** - The counting algorithm processes the ballots
4. **Aggregate** - Results from different areas/districts are combined
5. **Mark Winners** - Winning candidates are determined
6. **Generate Reports** - PDF and CSV reports are created
7. **Output** - Results are stored and made available

## Integration with Admin Portal

When an election administrator clicks "Execute Tally" in the Admin Portal:

1. The Admin Portal sends a request to the backend service (Windmill)
2. Windmill prepares the input data and configuration
3. Velvet is invoked as a library to process the tally
4. Results are stored in the database
5. Reports and receipts are generated
6. The Admin Portal displays the results

This integration allows for secure, auditable, and verifiable election tallying within the platform.

## Design Documentation

For more detailed design information, see the [Velvet design document](https://github.com/sequentech/step/blob/main/docs/design/velvet/README.md).

## Package Location

Velvet is located at: `/packages/velvet/`
