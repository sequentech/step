---
id: electoral-log
title: Electoral Log tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Test file paths in this guide are relative to [`packages/electoral-log/tests/`](https://github.com/sequentech/step/blob/main/packages/electoral-log/tests). Commands state their working directory.

From the repository root, run the tests that need no database:

```bash
cd packages
cargo test -p electoral-log --locked
```

For the complete profile, install [ImmuDB 1.9.6](https://github.com/codenotary/immudb/releases/tag/v1.9.6),
the version used by this repository. Put `immudb` on PATH, or set
`ELECTORAL_LOG_TEST_IMMUDB_BINARY` to the executable's absolute path. From the
repository root, run:

```bash
cd packages
cargo test -p electoral-log --features immudb-tests --locked
cd ..
python3 scripts/coverage/run.py electoral-log
```

Each database test starts its own ImmuDB process on a local ephemeral port with
synthetic credentials and a temporary data directory. It does not accept a
server URL or deployment credentials. The child is stopped and reaped even if a
test fails. The database scenarios have a two-minute deadline; they fail if the
binary is missing. The legacy ignored fixed-port test is preserved, but the new
suite covers its behavior without requiring an existing database.

The suite covers:

- Real sender/system signatures, altered statements, JSON/Borsh interchange,
  fixed wire tags, malformed encodings, every truncated message prefix and
  writers that fail at each payload boundary.
- Required and optional database fields, reordered columns, nulls, duplicate or
  missing fields, invalid labels and inconsistent row dimensions.
- ImmuDB schema creation, insertion, nullable metadata, failed transactions,
  filtering, epoch-zero bounds, sorting and both pagination strategies across
  903 records. The helper CLI also runs against the owned database.
- Stream failures after valid rows: a failed or malformed response is an error,
  never a successful partial audit page.
- Transaction retries accept only ImmuDB's typed `Unknown` / `tx read conflict`
  rejection, with at most five retries. Native tests check error classification
  and the attempt limit; an owned-database test forces a competing commit and
  verifies that the original batch persists exactly once. Windmill retries only
  the current board with a fresh session and transaction. Exhausted retries and
  other failures leave the original durable RabbitMQ delivery unacknowledged.
  Each board stores a receipt with its audit rows in the same transaction; a
  redelivery after a partial or uncertain commit therefore skips committed rows.
  The stable correlation ID and original input fingerprint identify a delivery,
  including legacy batch task IDs with an event index. Reusing an ID with a
  different input is rejected and retained for operator investigation.

The existing protocol signs the statement. Search metadata and the separate
artifact are not authenticated by `Message::verify`; these tests make no broader
claim. Full election workflows, server persistence under power loss and native
branch coverage require separate evidence.

All unit tests live outside `src`, including the preserved existing tests, so
their bodies do not inflate source coverage. Derive-generated methods remain in
LLVM's measured totals.

Row contracts give every optional field a distinct value, including an
empty ballot ID, so accidental field swaps or normalization are observable. A
valid explicit null must not hide a duplicate column from a joined table. The separate `electoral-log-native` profile measures default features on both
revisions for the strict per-metric CI comparison. Use it when the actual base
does not provide the `immudb-tests` feature; retain full-profile evidence separately. It does not claim database
integration coverage. Neither missing feature support nor an empty report is
converted into a zero baseline.

Sorting on nonunique timestamps or metadata appends `id ASC` unless callers
already specify the ID direction. A literal SQL regression and real tied-row
pagination controls check both default and explicit tie-breaking. No database ordering is inferred from insertion luck.

Windmill owns the AMQP channel until all boards commit. Returned errors close it;
cancellation drops it, causing RabbitMQ to requeue unacknowledged input. Session
cleanup errors after a confirmed commit do not prevent acknowledgement. Existing
worker arguments for `electoral_log_batch_queue` or `electoral_log_event_queue`
select the dispatcher queue; the dispatcher drains both current single events and
legacy queued batches. Stop old workers when deploying this change so they cannot
consume these queues with the old early-acknowledgement behavior. Already lost
messages from older workers cannot be reconstructed. An old dispatcher may also
have published a batch before crashing without ACKing its originals; those two
envelopes have different IDs and cannot be deduplicated retrospectively. The
dispatcher has no fixed preparation deadline, matching the former batch processor,
so a large valid batch is not repeatedly cancelled before its first commit.
Prepared deliveries are committed in groups of at most sixteen per board, keeping
both audit rows of a communications delivery with its receipt and leaving room
for ImmuDB's SQL indexes. A later chunk failure leaves earlier receipts available
for safe redelivery. The owned-database regression exercises a full thousand-event
batch, complete replay and a connection failure after the first committed chunk.
A permanently invalid input
stays queued and can block progress until investigated; it is never silently ACKed.

The broker regressions use owned RabbitMQ 3.12.11 containers on ephemeral loopback
ports and the owned ImmuDB fixture. They check sink errors, cancellation, legacy
batches, partial and uncertain commits, and six real conflicts followed by successful
redelivery. Docker must be available (set `WINDMILL_TEST_DOCKER_SUDO=1` only when
local Docker access requires noninteractive sudo). From the repository root:

```bash
cd packages
cargo test --locked -p windmill --features rabbitmq-tests --test electoral_log_delivery
```

The Windmill CI job runs these contracts separately from its native unit profile.
The receipt table is additive and created on existing boards before delivery;
backups and restores must retain it alongside the audit rows to retain deduplication.
