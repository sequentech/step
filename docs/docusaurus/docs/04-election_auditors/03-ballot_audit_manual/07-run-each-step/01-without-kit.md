---
title: Without the Audit Image
sidebar_position: 1
description: "Build the tools with Rust and restore the database copies by hand, without the audit image."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Without the Audit Image

This procedure builds the tools and restores the copies by hand. Use it when you cannot use the
audit image, or to check what the image does. It replaces
[Set Up the Audit Workstation](../02-procedures/01-workstation.md#1-get-the-audit-image) step 1
and step 2 of [Run the Audit](../02-procedures/03-run-the-audit.md).

| Database | Content |
| --- | --- |
| Hasura (PostgreSQL) | The election event, the tally, the stored ballots and their history. |
| Keycloak (PostgreSQL) | The voters, their area and their status. |
| B3 (PostgreSQL) | The bulletin board. |
| ImmuDB | The electoral log. |

## Before you start

- You have the source code at the agreed commit. See
  [Set Up the Audit Workstation](../02-procedures/01-workstation.md#1-get-the-audit-image), "If
  you build the image from the source code", steps 1 to 4.
- You did step 2 of [Set Up the Audit Workstation](../02-procedures/01-workstation.md).
- The Rust toolchain of the release is installed. The repository file `rust-toolchain.toml` gives
  the version.
- You have the four backups and their digests. See
  [Prepare the Backups](../02-procedures/02-restore.md).
- You have the password of the ImmuDB admin user of the source, from the system operator.

## 1. Build the tools

1. Go to the `packages` directory:

   ```sh
   cd packages
   ```

2. Build the six ballot audit commands:

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

**Expected result:** the `target/release` directory contains `audit-tally`,
`export-ballot-audit`, `export-electoral-log`, `prepare-ballot-audit`, `ballot-audit`,
`audit-cast-history`, `server` and `verify`.

:::note
`audit-tally` runs the other commands and `verify` from its own directory. Keep all of them in
`target/release`.
:::

## 2. Check the backups

1. Put the backups in `~/audit/backups`.
2. Calculate the digest of each backup:

   ```sh
   shasum -a 512 ~/audit/backups/*
   ```

3. Compare each digest with the digest from the system operator.
4. Write the file names and the digests in the audit notes.

:::danger WARNING
If a digest is different, stop. Do not restore the backup. Tell the election authority.
:::

**Expected result:** each digest agrees with the digest from the system operator.

## 3. Restore the PostgreSQL copies

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
8. Read the output of `pg_restore`. Write each error in the audit notes.

**Expected result:** `pg_restore` shows no errors. The three databases contain the restored
data.

## 4. Make the read-only accounts

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

## 5. Write the connections file

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

## 6. Restore the ImmuDB copy

1. Make a Docker volume and extract the ImmuDB archive into it:

   ```sh
   docker volume create audit-immudb
   ```

   ```sh
   docker run --rm -v audit-immudb:/dst -v ~/audit/backups:/src:ro alpine tar -C /dst -xzf /src/immudb.tar.gz
   ```

2. Start ImmuDB on the volume, with the same version as the source. Publish the port on the
   loopback address only:

   ```sh
   docker run -d --name audit-immudb --network audit-net -p 127.0.0.1:43322:3322 \
     -v audit-immudb:/var/lib/immudb codenotary/immudb:<version>
   ```

3. Find the name of the electoral log database. It is the name of the event board of the tally:

   ```sh
   docker exec audit-hasura-db psql -U audit_reader -d hasura_copy -c \
     "SELECT e.bulletin_board_reference->>'database_name' AS board_name
      FROM sequent_backend.election_event e
      JOIN sequent_backend.tally_session t ON t.election_event_id = e.id
      WHERE t.id = '<tally ID>'"
   ```

4. Make a read-only ImmuDB user for this database, with `immuadmin`.

**Expected result:** ImmuDB starts and answers on `127.0.0.1:43322`.

## 7. Start the B3 server

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

## 8. Run audit-tally

Do [Run the Audit](../02-procedures/03-run-the-audit.md) with this command in step 2, in
`~/audit`. Read the results in `~/audit/results/tally`. First make the results directory and set the ImmuDB password:

```sh
cd ~/audit && mkdir -m 700 results
```

```sh
export IMMUDB_READ_ONLY_USER=<user>
read -r -s BALLOT_AUDIT_IMMUDB_PASSWORD && export BALLOT_AUDIT_IMMUDB_PASSWORD
```

```sh
audit-tally --connections connections.json \
  --tally-session-id "$TALLY_SESSION_ID" \
  --public-key-sha512 "$PUBLIC_KEY_SHA512" \
  --electoral-log-system-public-key "$ELECTORAL_LOG_SYSTEM_PUBLIC_KEY" \
  --immudb-server-url http://127.0.0.1:43322 --immudb-username "$IMMUDB_READ_ONLY_USER" \
  --b3-server-url http://127.0.0.1:50051 \
  --output results/tally
```

If the bulletin board stores messages in files, add `--b3-blob-root <path>`.

## If there is a problem

| Problem | Action |
| --- | --- |
| `cargo` is not found. | Install the Rust toolchain of the release. The repository file `rust-toolchain.toml` gives the version. |
| The build stops with a missing system library. | Install the library that the message names, then build again. |
| The build changes `Cargo.lock`. | Do not commit the change. Write the difference in the audit notes. |
| `pg_restore` shows "already exists" errors. | The container image made objects at start. Delete the database, make it again from `template0` and restore again. |
| ImmuDB does not start. | Make sure that the ImmuDB version is the version of the source. |
| The B3 server ignores a setting. | Set the value with a `B3_` environment variable. The server has no command-line options. |

## Next

[Run the Audit](../02-procedures/03-run-the-audit.md)
