---
title: Set Up the Audit Workstation
sidebar_position: 1
description: "Prepare an isolated computer with Docker and get the audit image, which contains all tools of the audit and the database servers of the copies."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Set Up the Audit Workstation

This procedure prepares a computer for the audit. The auditor does it once for each audit. The
computer needs only Docker and the audit image. The audit image contains all tools of the audit
and the database servers that hold the copies.

## Before you start

- You have a Linux or macOS computer with at least 16 GB of memory and enough disk space for
  the backups and for the four database copies.
- The computer has encrypted storage.
- Docker is installed.
- You know the release of the election platform that ran the election. See
  [Versions](../01-before-you-start.md#versions).

## 1. Get the audit image

:::note
The shell examples use Linux or macOS syntax. The reviewed image was tested on Linux amd64 only.
Confirm compatibility before using arm64 or Docker Desktop on macOS.
See [Versions](../01-before-you-start.md#versions) for the implementation and test status.
:::

Do one of these two tasks.

**If the election authority gives you the image as a file:**

1. Load the image:

   ```sh
   docker load -i <image file>
   ```

2. Set the name of the image that the command shows:

   ```sh
   export AUDIT_IMAGE=<image name>
   ```

**If you build the image from the source code:**

:::note
The build needs access to the internet. Build the image before you copy the backups to the
computer. The audit itself does not need a network.
:::

1. Clone the source repository and go to it:

   ```sh
   git clone https://github.com/sequentech/step.git && cd step
   ```

2. Check out the release tag or commit that the election authority gives you:

   ```sh
   git checkout <tag-or-commit>
   ```

3. Make sure that the commit contains the audit image:

   ```sh
   test -f packages/ballot-audit/image/run-audit && echo found
   ```

4. Write the commit in the audit notes:

   ```sh
   git rev-parse HEAD
   ```

5. Set the name of the image. It contains the commit:

   ```sh
   export AUDIT_IMAGE=ballot-audit:$(git rev-parse --short HEAD)
   ```

6. Build the image:

   ```sh
   docker build -f packages/ballot-audit/Dockerfile.prod -t "$AUDIT_IMAGE" packages
   ```

**Then, in both cases:**

1. Write the ID of the image in the audit notes:

   ```sh
   docker image inspect --format '{{.Id}}' "$AUDIT_IMAGE"
   ```

2. Ask a second person to compare the ID with the ID from the election authority, if there is
   one.

**Expected result:** `docker run --rm "$AUDIT_IMAGE" help` shows how to run the audit.

:::note
The image contains `audit-tally`, `export-ballot-audit`, `export-electoral-log`,
`prepare-ballot-audit`, `ballot-audit`, `audit-cast-history`, the B3 server, the Braid verifier
`verify`, PostgreSQL 18 and ImmuDB 1.9.6. To build the tools without Docker, see
[Without the Audit Image](../07-run-each-step/01-without-kit.md).
:::

## 2. Make the audit directory

1. Make a private directory for the audit, its backups and its output:

   ```sh
   mkdir -m 700 ~/audit ~/audit/backups ~/audit/output
   ```

2. Make a text file `audit-notes.txt` in `~/audit`. Write in it each value that this manual tells
   you to record.

**Expected result:** you have a private audit directory with an empty audit notes file.

## If there is a problem

| Problem | Action |
| --- | --- |
| The command does not show `found`. | The commit is from before the audit image. Ask the election authority for a commit that contains `packages/ballot-audit/image`. |
| `docker` is not found, or it cannot connect to the Docker daemon. | Install Docker and start it. |
| The build stops because it cannot download a file. | Connect the computer to the internet for the build. Then disconnect it. |

## Next

[Prepare the Backups](02-restore.md)
