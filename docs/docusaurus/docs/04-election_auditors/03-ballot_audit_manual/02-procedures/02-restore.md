---
title: Restore the Database Copies
sidebar_position: 2
description: "Restore copies of the Hasura, Keycloak, B3 and ImmuDB databases on the audit workstation, with read-only accounts and no network access."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Restore the Database Copies

This procedure restores copies of the four databases of the election on the audit workstation.
The auditor does it. The tools read only these copies.

| Database | Content |
| --- | --- |
| Hasura (PostgreSQL) | The election event, the tally, the stored ballots and their history. |
| Keycloak (PostgreSQL) | The voters, their area and their status. |
| B3 (PostgreSQL) | The bulletin board. |
| ImmuDB | The electoral log. |

## Before you start

- You did [Set Up the Audit Workstation](01-workstation.md).
- You have the four backups from the system operator: a `pg_dump` custom-format file for each
  PostgreSQL database and an archive of the ImmuDB data directory.
- You have the SHA-512 digest of each backup from the system operator, through a channel that is
  not the channel of the backups.
- If the bulletin board stores messages in files, you also have a copy of the B3 blob directory.

:::note
Use backups from after the end of the tally. The audit compares the saved tally with the stored
data. A change after the backup of one database and before the backup of another makes the
copies disagree.
:::

## 1. Check the backups

1. Put the backups in `~/audit/backups`.
2. Calculate the digest of each backup:

   ```sh
   shasum -a 512 ~/audit/backups/*
   ```

3. Compare each digest with the digest from the system operator.
4. Write the file names and the digests in the audit record.

:::danger WARNING
If a digest is different, stop. Do not restore the backup. Tell the election authority.
:::

**Expected result:** each digest agrees with the digest from the system operator.

## 2. Restore the PostgreSQL copies

1. Make a Docker network for the copies:

   ```sh
   docker network create audit-net
   ```

2. Start one PostgreSQL container for each database. Use the same major version of PostgreSQL
   as the source. Publish the port on the loopback address only:

   ```sh
   docker run -d --name audit-hasura-db --network audit-net -p 127.0.0.1:55432:5432 \
     -e POSTGRES_PASSWORD="$LOCAL_PG_PASSWORD" postgres:<version>
   ```

3. Do step 2 again for `audit-keycloak-db` on port `55433` and `audit-b3-db` on port `55434`.
4. In each container, make an empty database from `template0`:

   ```sh
   docker exec audit-hasura-db createdb -U postgres -T template0 hasura_copy
   ```

5. Copy the backup into the container:

   ```sh
   docker cp ~/audit/backups/hasura.dump audit-hasura-db:/tmp/hasura.dump
   ```

6. Restore the backup:

   ```sh
   docker exec audit-hasura-db pg_restore -U postgres -d hasura_copy --no-owner --no-privileges /tmp/hasura.dump
   ```

7. Do steps 4 to 6 again for `keycloak_copy` and `b3_copy`.
8. Read the output of `pg_restore`. Write each error in the audit record.

**Expected result:** `pg_restore` shows no errors. The three databases contain the restored
data.

## 3. Make the read-only accounts

1. In each PostgreSQL container, make the role `audit_reader`:

   ```sh
   docker exec audit-hasura-db psql -U postgres -c "CREATE ROLE audit_reader LOGIN PASSWORD '<password>'"
   ```

2. Give the role read access to all data:

   ```sh
   docker exec audit-hasura-db psql -U postgres -c "GRANT pg_read_all_data TO audit_reader"
   ```

3. Do steps 1 and 2 again in `audit-keycloak-db` and `audit-b3-db`.

**Expected result:** each copy has a read-only account `audit_reader`.

## 4. Write the connections file

1. Make the file `~/audit/connections.json`. Use the ports, the database names and the backup
   identities of your copies:

   ```json
   {
     "format_version": 1,
     "hasura": {
       "connection_string": "host=127.0.0.1 port=55432 dbname=hasura_copy user=audit_reader password=<password>",
       "restore_reference": "<identity and digest of the Hasura backup>"
     },
     "keycloak": {
       "connection_string": "host=127.0.0.1 port=55433 dbname=keycloak_copy user=audit_reader password=<password>",
       "restore_reference": "<identity and digest of the Keycloak backup>"
     },
     "b3": {
       "connection_string": "host=127.0.0.1 port=55434 dbname=b3_copy user=audit_reader password=<password>",
       "restore_reference": "<identity and digest of the B3 backup>"
     }
   }
   ```

2. Make the file private:

   ```sh
   chmod 600 ~/audit/connections.json
   ```

:::caution CAUTION
Write a password only in this file. Do not put a password in a command, because the shell history
keeps it.
:::

**Expected result:** only your account can read `connections.json`.

## 5. Restore the ImmuDB copy

1. Make a Docker volume and extract the ImmuDB archive into it:

   ```sh
   docker volume create audit-immudb
   ```

   ```sh
   docker run --rm -v audit-immudb:/dst -v ~/audit/backups:/src:ro alpine tar -C /dst -xzf /src/immudb-data.tar.gz
   ```

2. Start ImmuDB on the volume, with the same version as the source. Publish the port on the
   loopback address only:

   ```sh
   docker run -d --name audit-immudb --network audit-net -p 127.0.0.1:43322:3322 \
     -v audit-immudb:/var/lib/immudb codenotary/immudb:<version>
   ```

3. Make a read-only ImmuDB user for the electoral log database, with `immuadmin`. The name of the
   database is the name of the event board. See [Get the Scope and the Pins](03-scope.md).

**Expected result:** ImmuDB starts and answers on `127.0.0.1:43322`.

## 6. Start the B3 server

The Braid verifier reads the bulletin board through a B3 server.

1. Start the B3 server on the B3 copy:

   ```sh
   B3_PG_HOST=127.0.0.1 B3_PG_PORT=55434 B3_PG_USER=postgres B3_PG_PASSWORD="$LOCAL_PG_PASSWORD" \
     B3_PG_DATABASE=b3_copy B3_BIND=127.0.0.1:50051 server
   ```

2. If you have a B3 blob directory, add `B3_BLOB_ROOT=<path>` to the command.

:::note
The B3 server makes an index and can write cached files when it starts. This is why it uses the
copy and the account `postgres`, not `audit_reader`. Never start it on an original backup.
:::

**Expected result:** the B3 server listens on `127.0.0.1:50051`.

## If there is a problem

| Problem | Action |
| --- | --- |
| `pg_restore` shows "already exists" errors. | The container image made objects at start. Delete the database, make it again from `template0` and restore again. |
| ImmuDB does not start. | Make sure that the ImmuDB version is the version of the source. |
| The B3 server ignores a setting. | Set the value with a `B3_` environment variable. The server has no command-line options. |

## Next

[Get the Scope and the Pins](03-scope.md)
