---
id: velvet_pipes
title: Pipes
sidebar_position: 2
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Pipes

Pipes are the fundamental processing units in Velvet. Each pipe performs a specific transformation or operation on election data.

## Overview

Pipes are designed to be composable and chainable, allowing complex workflows to be built from simple, focused components. Each pipe:

- Takes input from a specified directory
- Performs a specific operation
- Writes output to a designated directory
- Logs its execution

## Available Pipes

### decode_ballots

Decodes encrypted ballots into plaintext votes that can be tallied.

**Location**: `/packages/velvet/src/pipes/decode_ballots/`

*Documentation to be added.*

### do_tally

Executes the counting algorithm on decoded ballots to produce tally results.

**Location**: `/packages/velvet/src/pipes/do_tally/`

*Documentation to be added.*

### generate_db

Creates a SQLite database containing the tally results for efficient querying and reporting.

**Location**: `/packages/velvet/src/pipes/generate_db/`

*Documentation to be added.*

### generate_reports

Produces PDF and CSV reports containing election results in human-readable formats.

**Location**: `/packages/velvet/src/pipes/generate_reports/`

*Documentation to be added.*

### mark_winners

Analyzes tally results to determine which candidates have won according to the electoral rules.

**Location**: `/packages/velvet/src/pipes/mark_winners/`

*Documentation to be added.*

### vote_receipts

Generates individual vote receipts that voters can use to verify their votes were counted.

**Location**: `/packages/velvet/src/pipes/vote_receipts/`

*Documentation to be added.*

## Pipe Configuration

Pipes are configured through the Velvet configuration file, which defines:

- The execution order of pipes
- Pipe-specific configuration options
- Input and output directories

Example configuration:

```json
{
  "version": "0.0.0",
  "stages": {
    "order": ["main"],
    "main": {
      "pipeline": [
        {
          "id": "decode-ballots",
          "pipe": "VelvetDecodeBallots",
          "config": {}
        },
        {
          "id": "do-tally",
          "pipe": "VelvetDoTally",
          "config": {
            "invalidateVotes": "Fail"
          }
        }
      ]
    }
  }
}
```

## Implementation

Each pipe implements the `Pipe` trait, which defines:

- `exec()` - Main execution method
- `input()` - Load and validate input data
- `output()` - Write results to output directory

This trait-based design ensures consistency across all pipes and makes it easy to add new pipes to the system.
