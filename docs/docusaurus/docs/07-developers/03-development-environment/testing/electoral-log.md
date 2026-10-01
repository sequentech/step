---
id: electoral-log
title: Electoral Log tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The signed-message and wire-format contracts remain unchanged. From `packages/`, run:

```bash
cargo test --locked -p electoral-log
```

For storage tests, set `ELECTORAL_LOG_TEST_DATABASE_URL` to a disposable PostgreSQL database owned by the test user. Tests initialize the schema and use unique board names. Run serially because schema initialization can race on an empty database:

```bash
cargo test --locked -p electoral-log --features postgres-tests -- --test-threads=1
```

These contracts cover raw signed bytes and optional metadata, board isolation and deletion, rollback on invalid/streamed input, concurrent idempotency, serialization before cursor allocation, filters, visibility, counts and complete pagination. They also check that reusing a delivery ID with changed input fails atomically, while a retry retains the originally confirmed bytes.

The existing protocol signs the statement. Search metadata and the separate artifact are not authenticated by `Message::verify`; these tests make no broader claim. PostgreSQL does not provide ImmuDB history proofs. See [storage and rollout](../electoral-log-postgres.md).

OVCS keeps ownership of the AMQP channel until every board commits. Errors and cancellation requeue unacknowledged inputs. The dispatcher drains both current single events and legacy batches. Each board batch is one transaction; there is no ImmuDB chunk limit. A permanently invalid input remains queued for investigation and can block progress.

Broker regressions use owned RabbitMQ 3.12.11 containers on ephemeral loopback ports and the disposable PostgreSQL database. They cover sink errors, cancellation, legacy batches, partial and uncertain commits, a thousand-event/two-thousand-row batch, complete replay and payload conflicts. Docker must be available:

```bash
cargo test --locked -p windmill --features rabbitmq-tests --test electoral_log_delivery -- --test-threads=1
```

The Windmill CI job runs these contracts with PostgreSQL. The `electoral-log` coverage profile enables `postgres-tests`; `electoral-log-native` measures default features for the existing baseline comparison. Neither skipped database tests nor missing baseline features count as passing integration coverage. Coverage bodies remain outside `src`.
