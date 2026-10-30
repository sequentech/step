---
id: developers_windmill
title: Developers Windmill
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Electoral log queue

Keycloak and Windmill publish electoral log events to
`<ENV_SLUG>_electoral_log_event_queue`. The `electoral_log_batch_dispatcher`
beat task reads them in batches of `DEFAULT_SQL_BATCH_SIZE`, writes them to the
election event's board and acknowledges each event only after its board's
immudb transaction is committed.

An event that can never be written is copied to the durable
`<ENV_SLUG>_electoral_log_dead_letter_queue` and removed from the event queue.
This covers messages that cannot be decoded, invalid tenant or election event
IDs, election events that no longer exist or have no board, and entries that
cannot be built. Windmill logs the reason as
`Moving electoral log event to the dead-letter queue`. Nothing consumes the
dead-letter queue: monitor its depth, and once the cause is fixed, move its
messages back to the event queue, for example with a RabbitMQ shovel.

When the Hasura or Keycloak database, the vault or immudb cannot be reached,
the affected events stay in the event queue and the next dispatcher run
retries them.

Each event is written with a receipt in the board's
`electoral_log_delivery_receipts` table, in the same immudb transaction as its
entries. The receipt's ID is derived from the message's AMQP correlation ID,
which Windmill and Keycloak set to the task ID and which stays the same when
RabbitMQ delivers the message again. An event that was written but not
acknowledged, for example because the worker stopped, is therefore not written
twice: the next run finds its receipt and only acknowledges it. A message
without a correlation ID, or a correlation ID reused for a different event, is
moved to the dead-letter queue. Receipts are added to existing boards on their
next write; no migration is needed.

Batches queued by earlier versions in `<ENV_SLUG>_electoral_log_batch_queue`
are drained by the same dispatcher and acknowledged once all of their events
are written. Workers started with `electoral_log_event_queue` or
`electoral_log_batch_queue` in their `consume -q` list consume
`electoral_log_beat_queue` instead, so Celery never acknowledges an electoral
log event that was not written. Existing deployment arguments need no change.

