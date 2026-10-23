---
title: Set Up the Audit Workstation
sidebar_position: 1
description: "Prepare an isolated computer and build the ballot audit tools, the B3 server and the Braid verifier."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Set Up the Audit Workstation

This procedure prepares an isolated computer for the audit and builds the tools. The auditor does
it once for each audit.

## Before you start

- You have a Linux or macOS computer with at least 16 GB of memory and enough disk space for
  the four database copies.
- The computer has encrypted storage.
- Docker, Git, the Rust toolchain and the `shasum` command are installed.
- You know the release of the election platform. The tools must be from the same release line.

:::danger WARNING
Use a computer that cannot reach the production network. The copies contain personal data of
the voters.
:::

## 1. Get the source code

1. Clone the source repository:

   ```sh
   git clone https://github.com/sequentech/step.git
   ```

2. Go to the repository:

   ```sh
   cd step
   ```

3. Check out the release tag or commit that the election authority gives you:

   ```sh
   git checkout <tag-or-commit>
   ```

4. Write the commit in the audit record:

   ```sh
   git rev-parse HEAD
   ```

**Expected result:** the repository is at the agreed commit.

## 2. Build the tools

1. Go to the `packages` directory:

   ```sh
   cd packages
   ```

2. Build the five ballot audit commands:

   ```sh
   cargo build --release -p ballot-audit --bins
   ```

3. Build the B3 server:

   ```sh
   cargo build --release -p b3 --features server --bin server
   ```

4. Build the Braid verifier:

   ```sh
   cargo build --release -p braid --bin verify
   ```

5. Add the build directory to the `PATH` of your shell:

   ```sh
   export PATH="$PWD/target/release:$PATH"
   ```

**Expected result:** the `target/release` directory contains `export-ballot-audit`,
`export-electoral-log`, `prepare-ballot-audit`, `ballot-audit`, `audit-cast-history`, `server`
and `verify`.

## 3. Make the audit directory

1. Make a private directory for the audit:

   ```sh
   mkdir -m 700 ~/audit && cd ~/audit
   ```

2. Make a text file `audit-record.txt` in this directory. Write in it each value that this manual
   tells you to record.

**Expected result:** you have a private audit directory with an empty audit record.

## If there is a problem

| Problem | Action |
| --- | --- |
| `cargo` is not found. | Install the Rust toolchain of the release. The repository file `rust-toolchain.toml` gives the version. |
| The build stops with a missing system library. | Install the library that the message names, then build again. |
| The build changes `Cargo.lock`. | Do not commit the change. Write the difference in the audit record. |

## Next

[Restore the Database Copies](02-restore.md)
