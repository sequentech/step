---
title: Restore the Database Copies
sidebar_position: 2
description: "Check the backups and restore copies of the Hasura, Keycloak, B3 and ImmuDB databases with the audit kit, with read-only accounts and no network access."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Restore the Database Copies

This procedure restores copies of the four databases of the election on the audit workstation,
with one command, `audit-env up`. The auditor does it. The tools read only these copies.

| Database | Content |
| --- | --- |
| Hasura (PostgreSQL) | The election event, the tally, the stored ballots and their history. |
| Keycloak (PostgreSQL) | The voters, their area and their status. |
| B3 (PostgreSQL) | The bulletin board. |
| ImmuDB | The electoral log. |

The command does these steps:

1. It checks the digest of each backup. It stops if a digest is missing or different.
2. It restores each backup into a new copy.
3. It makes the account `ballot_audit_reader`, which can only read the copies.
4. It gives the ImmuDB copy a new admin password, so you do not need the password of the source.
5. It starts the B3 server on the B3 copy.
6. It writes the connections file `~/audit/private/connections.json`.

The copies run in containers that have no network. They can connect only to each other.

## Before you start

- You did [Set Up the Audit Workstation](01-workstation.md). `AUDIT_IMAGE` is set.
- You have these backups from the system operator:

  | File | Content |
  | --- | --- |
  | `hasura.dump` | The Hasura database, made with `pg_dump` in custom format. |
  | `keycloak.dump` | The Keycloak database, made with `pg_dump` in custom format. |
  | `b3.dump` | The B3 database, made with `pg_dump` in custom format. |
  | `immudb.tar.gz` | An archive of the ImmuDB data directory. |
  | `b3-blobs.tar.gz` | Only if the bulletin board stores messages in files: an archive of the B3 blob directory. |

- You have the SHA-512 digest of each backup from the system operator, through a channel that is
  not the channel of the backups.

:::note
Use backups from after the end of the tally. The audit compares the saved tally with the stored
data. A change after the backup of one database and before the backup of another makes the
copies disagree.
:::

## 1. Put the backups in place

1. Copy the backups to `~/audit/backups`. Use the file names of the table.
2. Write the file names in the audit notes.

:::caution CAUTION
Keep the original backups. The audit kit reads the files in `~/audit/backups` and does not change
them, but you need the originals if you must restore again.
:::

**Expected result:** `~/audit/backups` contains the backups.

## 2. Write the digests file

1. Make the file `~/audit/backup-digests.txt`. Write one line for each backup: the digest from the
   system operator, two spaces and the file name:

   ```text
   <digest of hasura.dump>  hasura.dump
   <digest of keycloak.dump>  keycloak.dump
   <digest of b3.dump>  b3.dump
   <digest of immudb.tar.gz>  immudb.tar.gz
   ```

2. Write the digests in the audit notes.

**Expected result:** the digests file has one line for each backup.

## 3. Restore the copies

:::danger WARNING
If the command shows `backup digest mismatch`, stop. Do not use the backup. Tell the election
authority.
:::

1. Restore the copies. Use the versions of the source:

   ```sh
   audit-env up --dir ~/audit --digests ~/audit/backup-digests.txt --image "$AUDIT_IMAGE" \
     --postgres-version <PostgreSQL major version> --immudb-version <ImmuDB version>
   ```

2. Read the messages of the command.

**Expected result:** the command shows `SHA-512 matches` for each backup, and then
`the copies are ready. They have no network access.`

## 4. Read the restore warnings

1. If the command shows `WARNING: pg_restore reported ... error(s)`, read the file that the
   warning names in `~/audit/private`.
2. Write each error in the audit notes.

:::note
The directory `~/audit/private` contains the passwords of the copies. Do not copy it. `audit-env
down` removes it.
:::

**Expected result:** the audit notes contain each restore error, or the command showed no
warning.

## If there is a problem

| Problem | Action |
| --- | --- |
| `missing backup: backups/...` or `no digest for ...` | Put the backup in `~/audit/backups` with its file name, and its digest in the digests file. |
| `copies already exist in ...; run audit-env down first` | A restore was done before. Run `audit-env down --dir ~/audit`, then restore again. |
| `image not found: ...` | Build the image. See [Set Up the Audit Workstation](01-workstation.md#2-build-the-audit-image). |
| `... did not start; see: docker compose -p ... logs` | Read the logs with the command that the message gives. Make sure that the versions are the versions of the source. Then run `audit-env down` and restore again. |
| `the ImmuDB archive has no systemdb directory` | The archive is not an ImmuDB data directory. Ask the system operator for the correct archive. |

See [Troubleshooting](../04-troubleshooting.md#messages-of-audit-env) for all messages.

## Next

[Run the Audit](03-run-the-audit.md)
