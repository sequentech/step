---
title: Prepare the Backups
sidebar_position: 2
description: "Put the backups of the Hasura, Keycloak, B3 and ImmuDB databases and their digests in one directory for the audit."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Prepare the Backups

This procedure puts the backups of the election and their digests in one directory. The auditor
does it. The audit command reads only this directory. It restores the backups into copies inside
the audit image.

## Before you start

- You have these backups from the system operator:

  <div className="table-names">

  | File | Content |
  | --- | --- |
  | `hasura.dump` | The Hasura database: the election event, the tally, the stored ballots and their history. Made with `pg_dump` in custom format. |
  | `keycloak.dump` | The Keycloak database: the voters, their area and their status. Made with `pg_dump` in custom format. |
  | `b3.dump` | The B3 database: the bulletin board. Made with `pg_dump` in custom format. |
  | `<database>.bkp` | A hot backup of each ImmuDB database, made with `immuadmin hot-backup`. The name can also be `<database>_backup_<YYYYMMDD>_<HHMMSS>.bkp`. The electoral log of the election event must be one of them. |
  | `b3-blobs.tar.gz` | Only if the bulletin board stores messages in files: an archive of the B3 blob directory. |

  </div>

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

**Expected result:** `~/audit/backups` contains the backups.

## 2. Write the digests file

1. Make the file `~/audit/backups/backup-digests.txt`. Write one line for each backup: the digest
   from the system operator, two spaces and the file name:

   ```text
   <digest of hasura.dump>  hasura.dump
   <digest of keycloak.dump>  keycloak.dump
   <digest of b3.dump>  b3.dump
   <digest of the hot backup>  <database>.bkp
   ```

   Write one line for each `.bkp` file.

2. Write the digests in the audit notes.

**Expected result:** the digests file has one line for each backup. The audit checks each backup
against it before it uses the backup.

## If there is a problem

| Problem | Action |
| --- | --- |
| A backup has another file name. | Rename the copy in `~/audit/backups`. Write the original name in the audit notes. Do not rename the `.bkp` files: their name gives the ImmuDB database. |
| The system operator cannot give a digest. | Stop. Without the digest, the audit cannot show that the backup is the backup of the system operator. |

## Next

[Run the Audit](03-run-the-audit.md)
