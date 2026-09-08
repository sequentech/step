---
title: Deploy the voting flow
sidebar_position: 6
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The portal downloads immutable ballot publications from private S3. `GetVoterStatus` supplies authorized object URLs, current eligibility, voting policy and cast metadata. Election listing downloads metadata; selecting an election downloads its ballot. Downloads are cached within the authenticated session, and expired URLs permit one authenticated renewal.

Cast validation remains authoritative in PostgreSQL. A single transaction reads scoped policy and records the vote; serialized voter checks enforce revote and cross-area rules. Voting windows are maintained transactionally when schedules change. Audit delivery remains asynchronous.

## Apply database changes

Run from the repository root against the intended environment, before voting peaks. The window backfill and index migrations take table write locks; allow time for existing data. Invalid or duplicate schedule endpoints abort the migration atomically and must be corrected before retrying.

```bash
read -r -p 'Hasura endpoint: ' HASURA_GRAPHQL_ENDPOINT
read -rs -p 'Hasura administrator secret: ' HASURA_GRAPHQL_ADMIN_SECRET
export HASURA_GRAPHQL_ENDPOINT HASURA_GRAPHQL_ADMIN_SECRET
hasura migrate apply \
  --project hasura \
  --database-name backend-db
```

| Migration | Purpose |
| --- | --- |
| `1788765000000` | Serialize eligibility and cross-area cast checks |
| `1788765000001` | Store encrypted ballots using PostgreSQL EXTERNAL storage |
| `1788765000002` | Backfill and maintain voting windows; index active schedules |
| `1788808561206` | Index authorized ballot-reference lookups |

The additional covering index is built separately, outside a transaction. Supply the writer connection using the standard PostgreSQL environment variables or a service file:

```bash
psql \
  --no-psqlrc \
  --set ON_ERROR_STOP=1 \
  --file scripts/postgres/cast_vote_covering_index.sql
```

The script builds a replacement concurrently before retiring the old index. After interruption, inspect both indexes before proceeding:

```sql
SELECT indexrelid::regclass, indisvalid
FROM pg_index
WHERE indexrelid IN (
  to_regclass('sequent_backend.cast_vote_participation_election_idx'),
  to_regclass('sequent_backend.cast_vote_participation_election_covering_idx')
);
```

If the replacement is invalid, drop only that replacement concurrently and rerun the script. If it is valid, complete the remaining drop/rename statements from the script. Never retire the valid old index after a failed build.

## Publish private ballot files

Deploy the backend with access to its private bucket. Set `AWS_S3_PUBLIC_URI` to the browser-reachable S3/CDN origin and allow the portal origin through bucket CORS. Keep active publication objects out of blanket expiration rules. The voter token mapper must provide event, area and authorized-election claims.

New publication generation uploads immutable objects, verifies their bytes and commits the reference only after every upload succeeds. Failed attempts leave the active publication unchanged. Prepare each existing active publication before deploying the portal, using the configured backend environment:

```bash
read -r -p 'Tenant ID: ' TENANT_ID
read -r -p 'Election event ID: ' EVENT_ID
read -r -p 'Active publication ID: ' PUBLICATION_ID
cargo run \
  --manifest-path packages/windmill/Cargo.toml \
  --example prepare_ballot_files \
  -- "$TENANT_ID" "$EVENT_ID" "$PUBLICATION_ID"
```

This command takes the publication lock and is a no-op for already prepared publications. If presentation configuration has changed since generation, regenerate and publish the intended configuration instead. Retain database ballot styles for writer compatibility and rollback.

Apply the action metadata after the backend is available, then deploy the portal:

```bash
hasura metadata apply \
  --project hasura
unset HASURA_GRAPHQL_ADMIN_SECRET
```

Unprepared publications fail closed. The portal never falls back to downloading ballot EML through GraphQL. Validate login, election listing and casting with a small [voting load test](./voter-status-performance.md).

## Roll back

Restore the previous portal to restore its database read path. Restore the previous backend before removing the voting-window projection it queries. Prefer retaining the strengthened eligibility trigger: its down migration removes database cross-area enforcement. Storage rollback affects future writes only. Private ballot objects can remain; remove unreferenced failed-attempt directories only after checking publication references.
