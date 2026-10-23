---
title: Export the Evidence
sidebar_position: 4
description: "Read the evidence of the scope from the restored copies into files, and record the digest of each file."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Export the Evidence

This procedure reads the evidence of the scope from the restored copies and writes it into files.
It also checks the scope against the saved tally. The auditor does it.

Two commands do the export:

- `export-ballot-audit` reads the Hasura, Keycloak and B3 copies.
- `export-electoral-log` reads the ImmuDB copy.

Both commands open read-only transactions and accept only loopback addresses.

## Before you start

- You did [Get the Scope and the Pins](03-scope.md). `$SCOPE_SHA512` is set.
- The three PostgreSQL copies and the ImmuDB copy run.
- The directories `~/audit/postgres-export` and `~/audit/immudb-export` do not exist. The
  commands make them.

## 1. Export the PostgreSQL evidence

1. Go to the audit directory:

   ```sh
   cd ~/audit
   ```

2. Run the export:

   ```sh
   export-ballot-audit --scope scope.json --scope-sha512 "$SCOPE_SHA512" \
     --connections connections.json --output postgres-export
   ```

3. If the bulletin board stores messages in files, add `--b3-blob-root <path>` to the command.
4. Read the exit status:

   ```sh
   echo $?
   ```

**Expected result:** the exit status is `0`. The directory `postgres-export` contains
`scope.json`, `cast-votes.json`, `eligible-voters.json`, `board-messages.json`,
`cast-vote-history.json`, `saved-mapping.json` and `acquisition.json`.

## 2. Export the electoral log

1. Set the name of the read-only ImmuDB user:

   ```sh
   export IMMUDB_READ_ONLY_USER=<user>
   ```

2. Type the password of the user. The shell does not show it:

   ```sh
   read -r -s BALLOT_AUDIT_IMMUDB_PASSWORD && export BALLOT_AUDIT_IMMUDB_PASSWORD
   ```

3. Run the export. The database is the board name of the scope:

   ```sh
   export-electoral-log --server-url http://127.0.0.1:43322 \
     --database "<board name>" --username "$IMMUDB_READ_ONLY_USER" \
     --election-event-id "<election event ID>" --output immudb-export
   ```

4. Read the exit status.

:::note
Do not add `--election-id`. Without it, the export contains the electoral log of the full election
event. The cast-vote history check needs the full log.
:::

**Expected result:** the exit status is `0`. The directory `immudb-export` contains
`electoral-log.json` and `electoral-log-provenance.json`.

## 3. Record the digests

1. Calculate the digests of the evidence files and keep them in variables:

   ```sh
   pin() { shasum -a 512 "$1" | cut -d' ' -f1; }
   export CAST_VOTES_SHA512=$(pin postgres-export/cast-votes.json)
   export ELIGIBLE_VOTERS_SHA512=$(pin postgres-export/eligible-voters.json)
   export BOARD_MESSAGES_SHA512=$(pin postgres-export/board-messages.json)
   export SAVED_MAPPING_SHA512=$(pin postgres-export/saved-mapping.json)
   export CAST_VOTE_HISTORY_SHA512=$(pin postgres-export/cast-vote-history.json)
   export ELECTORAL_LOG_SHA512=$(pin immudb-export/electoral-log.json)
   ```

2. Write the six digests in the audit record.
3. Ask a second person to sign the audit record. From now on, the digests are the pins of the
   evidence.

**Expected result:** the audit record has the six digests and a second signature.

## If there is a problem

| Problem | Action |
| --- | --- |
| Exit status `1`, `... differs from pinned scope` | The scope does not agree with the saved tally. Check each value of `scope.json` against the tally. |
| Exit status `4`, `combined weighted and delegated voting is unsupported` | The tools cannot audit this tally. Write this in the audit report. |
| Exit status `4`, `remote database connections are refused` | Use `127.0.0.1` in `connections.json`, not a host name. |
| Exit status `4`, `connections file must not grant group or other access` | Run `chmod 600 connections.json`. |
| Exit status `4`, `output directory already exists` | Use a new output directory. Do not delete evidence that you need. |
| Exit status `3`, `missing B3 batch` | The B3 copy or the blob directory does not have a batch of the scope. Check the restore and `--b3-blob-root`. |

See [Troubleshooting](../04-troubleshooting.md) for all messages.

## Next

[Prepare the Audit Bundle](05-prepare.md)
