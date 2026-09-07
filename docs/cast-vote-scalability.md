<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Authenticated voter-flow scalability (meta-12767)

## Assessment

Reducing synchronous work per voter is worthwhile: concurrent requests hold
connections while waiting on redundant reads, not just on the ballot write.
However, the ticket's throughput figures are estimates, not a capacity test of
this checkout. Ordinary votes now start `valid`; only Datafix votes need the
asynchronous eligibility pipeline. Audit delivery already queues a signed message
and is best-effort after commit. Its extra application transaction reloads the
signing context; it does not make the audit atomic with the ballot.

This change removes that reload, the second initial protocol-manager lookup,
and the Keycloak username lookup. The verified JWT supplies the optional username;
its absence remains representable as `None` in audit messages. The existing system
key and voter identity construct the same voter signer. Ordinary successful casts
use one application transaction against the writer. Datafix retains its separate
lease and connection lifecycle: holding a pool connection while waiting on that
lease would regress scalability and eligibility safety.

The database trigger is now the sole revote enforcement point, and also enforces
cross-area exclusivity under its existing `(tenant,event,election,voter)` advisory
lock. Both `valid` and `in-progress` consume revote slots; `discarded` does not.
Unlimited revotes still cannot cross areas. INSERT remains the final SQL statement
before COMMIT. Error handling reads the actual PostgreSQL exception instead of
matching tokio-postgres's generic `db error` display string.

## Configuration and response size

A five-second cache can admit votes after an unscheduled administrative pause,
channel disable, or reschedule. Knowing the old window boundaries does not solve
that problem. Instead, one narrow writer query returns policy and only matching
start/end tasks. It avoids transferring EML, receipts, statistics, or unrelated
tasks and retains the existing `generate_voting_period_dates` implementation.
This is one round trip/result row, **not one physical row read**: PostgreSQL still
reads scheduled-event rows. No stale acceptance decision is cached.

Existing closing-boundary semantics are preserved: without grace, one second
before close is accepted and one second after close is rejected. The instant
exactly at close is accepted by the existing `now > close` comparison. The ticket's
literal rejection throughout ±1 second would change election policy.

INSERT no longer fetches ciphertext from TOAST. The serialized response retains
the supplied content; other returned fields still come from PostgreSQL. The portal
also omits unused election EML from `GetElections`; the signed ballot remains in
`ballot_style.ballot_eml`.

## Migrations and deployment

Apply the two normal Hasura migrations before deploying the application:

- `1788765000000_serialize_cast_vote_area_checks`: atomic trigger replacement.
- `1788765000001_cast_vote_external_storage`: future ballots skip compression;
  existing rows are not rewritten. Rollback restores `EXTENDED` for future writes.

For the participation index, execute against the writer using the deployment's
normal PostgreSQL authentication:

```sh
psql -X -v ON_ERROR_STOP=1 -f scripts/postgres/cast_vote_covering_index.sql
```

Do **not** use `--single-transaction`. The script builds the covering replacement
concurrently, then retires the old index concurrently and restores its original
name. It is intentionally separate from normal transaction-wrapped migrations;
no deployment-wide migration atomicity setting changes. Run it once and stop on
errors. On interruption, inspect `pg_index.indisvalid` and index definitions:
if the covering build is invalid, remove that incomplete index concurrently and
restart; if valid, resume the remaining drop/rename steps. If the old index was
already retired, only the rename remains. Do not blindly rerun CREATE over an
existing name or remove a valid access path. To roll back, use the same build,
retire, rename sequence with the original four-key definition without INCLUDE.
No reporting index is removed by this work.

Application rollback can precede trigger rollback: the former duplicate precheck
is compatible with the strengthened trigger. Rolling back the trigger restores
the previous cross-area race, so retain it when possible.

## Reproducible validation

From `/workspaces/step` inside the repository devenv container:

```sh
devenv shell bash -- -c 'CAST_VOTE_RUN_RUST_TESTS=1 python3 scripts/test_cast_vote_scalability.py'
devenv shell bash -- -c 'cd packages && CARGO_TARGET_DIR=/workspaces/step/packages/windmill/rust-local-target cargo test -p windmill --lib services::insert_cast_vote::tests'
devenv shell bash -- -c 'cd packages/voting-portal && yarn test --runInBand'
```

The PostgreSQL script creates an isolated disposable cluster, applies the real
migrations, and runs independent concurrent connections. It checks bounded and
unlimited revotes, cross-area exclusivity, discarded votes, rollback/reapplication,
storage settings, concurrent index replacement, and EXPLAIN plans. Optional Rust
integration verifies error mapping, writer configuration freshness/tenant scope,
and the INSERT response against that database.

Observed locally on PostgreSQL 18:

- 12 simultaneous same-voter submissions: exactly 3 succeeded for a limit of 3.
- 12 submissions split across two areas, unlimited revotes: 6 succeeded in exactly
  one area; the other 6 returned the cross-area error.
- On a 100,000-row synthetic fixture, the covering count used `Index Only Scan`
  with 3 heap fetches before vacuum and 0 after. INCLUDE does not eliminate MVCC
  visibility checks on fresh pages during a voting spike.
- A 16 KB random ciphertext encoded as JSON/base64 occupied 21,354 bytes under
  both EXTENDED and EXTERNAL. This is a synthetic storage check, not a benchmark
  of every real ballot shape or of compression CPU.
- All 355 Windmill library tests passed (3 ignored), including nine focused
  voting-policy tests; all 70 portal tests passed. The separate database
  integration test passed, including actual INSERT response/error checks.
  `cargo check -p harvest` passed. Fresh GraphQL generation matches the changed document
  and operation type. Workspace formatting and REUSE checks passed.

Clippy encounters unchanged errors in `strand/src/shuffler_product.rs` (unsigned
comparisons); `--no-deps` encounters an existing `skip_all(keycloak_transaction)`
attribute error in `windmill/src/services/users.rs`. Portal `tsc --noEmit` encounters
an installed `minimatch` type-definition error. These are not reported as passing.

## Remaining deployment evidence

Tracked in [meta#13211](https://github.com/sequentech/meta/issues/13211).

Before claiming a production p99 or pool-capacity improvement, run the complete
login-to-cast flow on a seeded deployment with the same PgBouncer transaction-pool
configuration as production. Use `packages/loadtesting` with an explicit test URL
and seeded credentials; do not rely on its shared-environment default URL. Run
opening spike, lull, and pre-close spike against baseline and this branch with
identical data and resources. Record arrival rates, p50/p95/p99, error counts,
pool waits, prepared-statement errors, and the structured `duration_us` phases.

Collect `pg_stat_user_indexes` deltas (and the statistics reset timestamp) across
a real election before considering removal of any other index. Larger shared
configuration caches/materialization require transactional invalidation for all
administrative and scheduled writers. End-to-end load results and production
index-usage evidence are not claimed by the local regression suite.
