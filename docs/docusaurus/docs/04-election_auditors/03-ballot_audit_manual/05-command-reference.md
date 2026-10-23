---
title: Command Reference
sidebar_position: 5
description: "All options of the ballot audit commands and of the Braid verifier, and the format of the input files that you write."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Command Reference

This page lists all options of the six commands of the audit, in the order of the procedures. It
also gives the format of the two files that you write: `scope.json` and `connections.json`.

## Common rules

- All options are long options: `--option value`. The commands have no short options.
- `--help` shows the options of a command. `--version` is not available.
- An option with the suffix `-sha512` is a pin: the SHA-512 digest of a file, in 128 lowercase
  hexadecimal characters. Get the pin from the audit record, not from the file.
- An `--output` directory must not exist. The command makes it with access for your account only,
  and it does not overwrite a file.
- All ballot audit commands use the exit statuses `0` to `4`. See
  [Read the Results](03-read-the-results.md#exit-statuses).

In the tables, **Required** shows if you must give the option. **Default** in the description is the
value that the command uses when you do not give the option.

## export-ballot-audit

Reads the restored Hasura, Keycloak and B3 copies in read-only transactions and writes the evidence
of the scope. It checks the scope against the saved tally. See
[Export the Evidence](02-procedures/04-export.md).

| Option | Required | Description |
| --- | --- | --- |
| `--scope <file>` | Yes | The scope file. See [scope.json](#scopejson). |
| `--scope-sha512 <digest>` | Yes | The pin of the scope file. |
| `--connections <file>` | Yes | The connections file. See [connections.json](#connectionsjson). Only your account can have access to it. |
| `--b3-blob-root <directory>` | No | The restored B3 blob directory. Give it when the bulletin board stores messages in files. |
| `--statement-timeout-seconds <n>` | No | The time limit of each database query, in seconds. The minimum is `1`. Default: `120`. |
| `--output <directory>` | Yes | The new directory for the evidence. |

**Output files:** `scope.json`, `cast-votes.json`, `eligible-voters.json`, `board-messages.json`,
`cast-vote-history.json`, `saved-mapping.json`, `keycloak-source.json`, `eligibility-reasons.json`,
`board-source.json` and `acquisition.json`.

## export-electoral-log

Reads the signed cast-vote records of an election event from the restored ImmuDB copy in a
read-only transaction. See [Export the Evidence](02-procedures/04-export.md).

| Option | Required | Description |
| --- | --- | --- |
| `--server-url <url>` | Yes | The loopback address of the restored ImmuDB, for example `http://127.0.0.1:43322`. |
| `--database <name>` | Yes | The electoral log database. Its name is the board name of the scope. |
| `--username <user>` | Yes | The read-only ImmuDB user. |
| `--password-env <variable>` | No | The name of the environment variable that holds the password. Do not give the password itself. Default: `BALLOT_AUDIT_IMMUDB_PASSWORD`. |
| `--election-event-id <ID>` | Yes | The election event of the records. |
| `--election-id <ID>` | No | Exports the records of one election only. Do not use it when you check the cast-vote history. |
| `--output <directory>` | Yes | The new directory for the electoral log. |

**Output files:** `electoral-log.json` and `electoral-log-provenance.json`.

## prepare-ballot-audit

Selects the ballots that the tally must count and writes the audit bundle. It reads only files.
See [Prepare the Audit Bundle](02-procedures/05-prepare.md).

| Option | Required | Description |
| --- | --- | --- |
| `--scope <file>` | Yes | The scope file from the export: `postgres-export/scope.json`. |
| `--scope-sha512 <digest>` | Yes | The pin of the scope file. |
| `--cast-votes <file>` | Yes | `cast-votes.json` from the export. |
| `--cast-votes-sha512 <digest>` | Yes | The pin of `cast-votes.json`. |
| `--eligible-voters <file>` | Yes | `eligible-voters.json` from the export. |
| `--eligible-voters-sha512 <digest>` | Yes | The pin of `eligible-voters.json`. |
| `--board-messages <file>` | Yes | `board-messages.json` from the export. |
| `--board-messages-sha512 <digest>` | Yes | The pin of `board-messages.json`. |
| `--electoral-log <file>` | Yes | `electoral-log.json` from the electoral log export. |
| `--electoral-log-sha512 <digest>` | Yes | The pin of `electoral-log.json`. |
| `--cast-vote-history <file>` | Yes, if you do not give `--without-cast-vote-history` | `cast-vote-history.json` from the export. Give it with its pin. |
| `--cast-vote-history-sha512 <digest>` | With `--cast-vote-history` | The pin of `cast-vote-history.json`. |
| `--without-cast-vote-history` | No | Skips the history checks. You cannot use it with `--cast-vote-history`. Do not use it in an audit. Default: Off. |
| `--saved-mapping <file>` | No | `saved-mapping.json` from the export. Give it with its pin. The command then compares the selection with the counts of the saved tally. |
| `--saved-mapping-sha512 <digest>` | With `--saved-mapping` | The pin of `saved-mapping.json`. |
| `--output <directory>` | Yes | The new directory for the audit bundle. |

**Output files:** `manifest.json`, `configuration.borsh`, `cast-votes.json`, `messages.json`,
`electoral-log.json`, `anchors.json`, `provenance.json` and `selection.json`.

:::caution CAUTION
Do not use `--without-cast-vote-history` in an audit. Without the history, the command cannot
detect a removed revote or a changed vote order.
:::

## ballot-audit

Checks the audit bundle a second time, independently of the preparation, and looks up a receipt.
It writes the report to the standard output. See [Audit the Bundle](02-procedures/06-audit.md).

| Option | Required | Description |
| --- | --- | --- |
| `--manifest <file>` | Yes | `manifest.json` from the bundle. |
| `--manifest-sha512 <digest>` | Yes | The pin of the manifest, from the audit record. |
| `--cast-votes <file>` | Yes | `cast-votes.json` from the bundle. |
| `--configuration <file>` | Yes | `configuration.borsh` from the bundle. |
| `--configuration-sha512 <digest>` | Yes | The pin of the configuration, from the audit record. |
| `--messages <file>` | Yes | `messages.json` from the bundle. |
| `--electoral-log <file>` | Yes | `electoral-log.json` from the bundle. |
| `--ballot-id <receipt>` | No | Looks up one receipt. The command still checks all ballots. |
| `--selection <file>` | No | `selection.json` from the bundle. With it, the lookup gives the reason when the tally did not count a ballot. |

## audit-cast-history

Compares all stored ballots of the election with the signed records of the electoral log. It
writes the report to the standard output. See
[Check the Cast-Vote History](02-procedures/08-history.md).

| Option | Required | Description |
| --- | --- | --- |
| `--scope <file>` | Yes | The scope file. The command uses only the tenant, the election event, the election and the system public key. |
| `--scope-sha512 <digest>` | Yes | The pin of the scope file. |
| `--cast-vote-history <file>` | Yes | `cast-vote-history.json` from the export. |
| `--cast-vote-history-sha512 <digest>` | Yes | The pin of `cast-vote-history.json`. |
| `--electoral-log <file>` | Yes | `electoral-log.json` of the full election event. |
| `--electoral-log-sha512 <digest>` | Yes | The pin of `electoral-log.json`. |
| `--ballot-id <receipt>` | No | Shows only the groups of this receipt in the report. The command still checks the full history. |

## verify (Braid)

Checks the signatures, the configuration, the mix and the decryption of each batch on the
bulletin board. It reads the board through the B3 server. See
[Verify the Bulletin Board](02-procedures/07-braid.md).

| Option | Required | Description |
| --- | --- | --- |
| `--server-url <url>` | Yes | The address of the B3 server, for example `http://127.0.0.1:50051`. |
| `--board <name>` | Yes | The board name of the scope. |
| `--batches <n,n,...>` | No | The batches that must pass, separated by commas. The check fails if a batch in the list is not verified, or if a batch that is not in the list is verified. Give all batches of the board. |
| `--ballot-hash <hash>` | No | Not available. The verifier does not use it. |

## scope.json

The scope file tells the commands which tally, election, area and contest to audit, and which
keys to trust. All fields are required. See [Get the Scope and the Pins](02-procedures/03-scope.md).

| Field | Value |
| --- | --- |
| `format_version` | `1` |
| `scope.tenant_id` | The tenant ID. |
| `scope.election_event_id` | The election event ID. |
| `scope.election_id` | The election ID. |
| `scope.area_id` | The area ID. |
| `scope.tally_session_id` | The tally ID. |
| `scope.tally_session_contest_id` | The ID of the row of the tally for this area and contest. |
| `scope.board_name` | The name of the event board. |
| `scope.contest_id` | The contest ID, or `null` for a ballot that encrypts all contests together. |
| `scope.base_batch` | The base batch of the area and contest. |
| `scope.policy` | `unweighted`, `weighted` or `delegated`. |
| `scope.public_key_sha512` | The public key hash, from the election authority. |
| `scope.electoral_log_system_public_key` | The electoral log system public key, from the election authority. |
| `batches` | The list of batches of the scope. |

Use lowercase IDs. Do not change the file after you record its pin.

## connections.json

The connections file tells `export-ballot-audit` how to connect to each restored copy. See
[Restore the Database Copies](02-procedures/02-restore.md).

| Field | Value |
| --- | --- |
| `format_version` | `1` |
| `hasura`, `keycloak`, `b3` | One object for each database copy. |
| `<copy>.connection_string` | A PostgreSQL connection string to a loopback address, for example `host=127.0.0.1 port=55432 dbname=hasura_copy user=audit_reader password=<password>`. A host name, a remote address or `sslmode=require` is refused. |
| `<copy>.restore_reference` | The identity and the digest of the backup of this copy. The export writes it in `acquisition.json`. |

The commands refuse a field that is not in this list. Only your account can have access to the
file: run `chmod 600 connections.json`.
