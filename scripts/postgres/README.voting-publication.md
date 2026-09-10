<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Voting schedules and publication files

Voting reads use two indexed, canonical START/END schedule lookups. The
`1788765000002_validate_voting_schedules` migration installs the active-schedule
indexes and a CHECK constraint for voting-task scope, payload and RFC3339 dates.
It creates no projection table, maintenance functions or schedule triggers.
The existing per-voter/election revote trigger remains necessary.

## Migration rollout

Apply `1788765000002_validate_voting_schedules` and
`1788909000000_ballot_publication_snapshot` before exercising the new generation
flow. The Hasura migrations create their indexes normally within the migration
transaction. Index creation blocks writes until the transaction commits; allow
an appropriate maintenance window for populated tables.

Constraint installation validates existing active voting schedules and fails
atomically on malformed configuration or duplicate active voting tasks. Resolve
the invalid configuration before retrying; do not silently discard either
deadline. If a development database retains an invalid index from an earlier
concurrent build, remove that invalid index before retrying the migration.

These PR migrations have only been applied in development, not production.
Version `1788765000002` is therefore edited in place (with its directory renamed
from `materialize_voting_windows` to `validate_voting_schedules`). There is no
additional cleanup migration: a fresh installation never creates the obsolete
voting-window table, its five helper functions, or its three triggers.

Development databases that applied the earlier version must roll back that
version using the original migration files and then apply the revised version.
Editing or renaming files does not rerun an already recorded Hasura migration.
Previously generated publications without prepared files must be regenerated
under a new publication ID.

## Publication lifecycle

Generation commits signed styles and frozen event/election metadata together.
The snapshot table is internal and is not exposed through Hasura metadata or
loaded by voter requests. A retry reuses that database generation. Uploads use
a separate UUID per task lease under
`tenant-{tenant}/event-{event}/publication-{publication}/{attempt}`.

Immutable ballot uploads read eight styles per page, send at most four objects
concurrently, and hold no database connection while awaiting S3. Conditional writes and exact
readback protect immutable content. The lease is renewed between batches; an
expired or replaced worker cannot mark the publication ready. A final transaction
checks ownership and eligibility before recording the verified root.
Publishing requires readiness and serializes only the active-reference update
using `FOR NO KEY UPDATE`, which permits concurrent vote foreign-key checks.

Normal generation and publishing retain old S3 objects. Failed attempts are also
retained; no automatic garbage collector is introduced here. Existing event and
tenant deletion still remove their S3 prefixes. The legacy public
`election_event_config.json` retains its fixed path and overwrite semantics. Its
single mutable upload uses current event metadata under the final event lock,
with a 60-second timeout, so a delayed draft cannot overwrite newer configuration
or write after event deletion. This is the one S3 upload that retains a database
connection; private ballot activation is governed by the fenced database reference.

Document exports paginate and download only recognized document keys whose IDs
are authorized by the database. Publication objects and uncommitted documents
are excluded. Importing a clone requires new publication artifacts; this does
not add historical signed-publication restoration.

Voter requests read current authorization, active roots and voting policy, then
release their database connection before signing five-minute URLs. The two S3
endpoint clients share one process-level SDK configuration and its refreshable
credential cache.

Admin Preview uses frozen publication metadata and opens both event and election
status only in its exported payload. It commits the preview document before
reporting task completion and reuses that document on completion retries.

## Focused verification

Run `python3 scripts/test_cast_vote_scalability.py` and
`python3 scripts/test_ballot_publication_lifecycle.py` with PostgreSQL utilities
and psycopg available. Each starts and removes its own database cluster.

Run `python3 scripts/test_ballot_files.py` with disposable PostgreSQL utilities
and a development private/public S3 service configured. It creates random test
prefixes, verifies failure/retry, frozen metadata, partial publication, live
policy, document filtering and listing pagination, then removes its own S3 data.
It must never receive an application database DSN. The script limits Cargo to
one job; set `CARGO_TARGET_DIR` to an existing compatible build cache. Do not run
it alongside a watcher compiling the same workspace.
