---
id: velvet_cli
title: Command Line Interface
sidebar_position: 6
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Command Line Interface (CLI)

Velvet provides a command-line interface for running tallying operations directly from the terminal.

## Overview

The CLI is a secondary tool primarily used for:

- **Development** - Testing new features and algorithms
- **Debugging** - Troubleshooting tally issues
- **Testing** - Running integration tests
- **Standalone operation** - Processing tallies outside the main platform

The main functionality in production is triggered from the **Admin Portal** UI when administrators execute a tally.

## Basic Usage

```bash
velvet run {stage} {optional-pipe} \
  --config ./path/to/velvet-config.json \
  --input-dir ./path/to/input-dir \
  --output-dir ./path/to/output-dir
```

### Arguments

- **stage** - The stage to execute (e.g., `main`)
- **optional-pipe** - Specific pipe to run (e.g., `decode-ballots`). If omitted, all pipes in the stage are executed
- **--config** - Path to the Velvet configuration file
- **--input-dir** - Directory containing input data (configs and ballots)
- **--output-dir** - Directory where results will be written

## Configuration File

The configuration file defines the pipeline of pipes to execute:

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

## Input Directory Structure

Input directory must follow this structure:

```
./input-dir/default/
├── configs/
│   └── election__<uuid>/
│       ├── election-config.json
│       └── contest__<uuid>/
│           ├── contest-config.json
│           └── area__<uuid>/
│               └── area-config.json
└── ballots/
    └── election__<uuid>/
        └── contest__<uuid>/
            └── area__<uuid>/
                └── ballots.csv
```

## Output Directory Structure

Velvet creates output directories for each pipe:

```
./output-dir/
├── status.json
└── main/
    ├── decode-ballots/
    │   └── output.log
    ├── do-tally/
    │   ├── result.json
    │   └── output.log
    └── generate-report/
        └── output.log
```

## Examples

### Run Complete Pipeline

Execute all pipes in the main stage:

```bash
velvet run main \
  --config ./velvet-config.json \
  --input-dir ./input \
  --output-dir ./output
```

### Run Single Pipe

Execute only the decode-ballots pipe:

```bash
velvet run main decode-ballots \
  --config ./velvet-config.json \
  --input-dir ./input \
  --output-dir ./output
```

### Development Workflow

```bash
# Build Velvet
cargo build

# Run a specific pipe
cargo run --bin velvet -- run main decode-ballots \
  --config ./velvet-config.json \
  --input-dir ./input \
  --output-dir ./output
```

## Location

CLI implementation: `/packages/velvet/src/cli/`

*Further documentation to be added.*
