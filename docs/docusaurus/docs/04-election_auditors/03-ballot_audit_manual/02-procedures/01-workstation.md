---
title: Set Up the Audit Workstation
sidebar_position: 1
description: "Prepare an isolated computer, build the audit image and get the images of the database copies."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Set Up the Audit Workstation

This procedure prepares a computer for the audit and builds the audit image. The auditor does it
once for each audit. The audit image contains all tools of the audit. The audit kit,
`audit-env`, uses it to restore the copies and to run the audit.

## Before you start

- You have a Linux or macOS computer with at least 16 GB of memory and enough disk space for
  the four database copies.
- The computer has encrypted storage.
- Docker with Docker Compose version 2, Git and the `shasum` command are installed.
- You know the release of the election platform that ran the election, the major version of
  PostgreSQL and the version of ImmuDB. See [Versions](../01-before-you-start.md#versions).

:::note
The build needs access to the internet. Do this procedure before you copy the backups to the
computer. After this procedure, the audit does not need a network.
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

4. Make sure that the commit contains the ballot audit tools:

   ```sh
   test -d packages/ballot-audit/kit && echo found
   ```

5. Write the commit in the audit notes:

   ```sh
   git rev-parse HEAD
   ```

**Expected result:** the repository is at the agreed commit, and the command shows `found`.

## 2. Build the audit image

1. Set the name of the image. It contains the commit:

   ```sh
   export AUDIT_IMAGE=ballot-audit:$(git rev-parse --short HEAD)
   ```

2. Build the image:

   ```sh
   docker build -f packages/ballot-audit/Dockerfile.prod -t "$AUDIT_IMAGE" packages
   ```

3. Write the ID of the image in the audit notes:

   ```sh
   docker image inspect --format '{{.Id}}' "$AUDIT_IMAGE"
   ```

4. Get the images of the database copies. Use the versions of the source:

   ```sh
   docker pull postgres:<PostgreSQL major version>
   docker pull codenotary/immudb:<ImmuDB version>
   ```

5. Add the audit kit to the `PATH` of your shell:

   ```sh
   export PATH="$PWD/packages/ballot-audit/kit:$PATH"
   ```

**Expected result:** `audit-env help` shows the commands of the audit kit.

:::note
The image contains `audit-tally`, `export-ballot-audit`, `export-electoral-log`,
`prepare-ballot-audit`, `ballot-audit`, `audit-cast-history`, the B3 server, the Braid verifier
`verify` and `immuclient`. To build the tools without Docker, see
[Without the Audit Kit](../07-run-each-step/01-without-kit.md).
:::

## 3. Make the audit directory

1. Make a private directory for the audit and its backups:

   ```sh
   mkdir -m 700 ~/audit ~/audit/backups
   ```

2. Make a text file `audit-notes.txt` in `~/audit`. Write in it each value that this manual tells
   you to record.

**Expected result:** you have a private audit directory with an empty audit notes file.

## If there is a problem

| Problem | Action |
| --- | --- |
| The command does not show `found`. | The commit is from before the tools. Ask the election authority for a commit that contains `packages/ballot-audit/kit`. |
| `docker compose` is not found. | Install Docker Compose version 2. |
| The build stops because it cannot download a file. | Connect the computer to the internet for the build. Then disconnect it. |
| `docker pull` cannot find the version. | Check the version of the source with the system operator. |

## Next

[Restore the Database Copies](02-restore.md)
