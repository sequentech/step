---
id: datafix_voterview_integration
title: Datafix / VoterView vote synchronization
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Datafix / VoterView vote synchronization

Election events without a `datafix:id` annotation store cast votes as `valid`
immediately. They do not publish a VoterView task. A Datafix event must include
the complete Datafix annotation set; an incomplete configuration fails closed
before the vote is stored.

For a configured Datafix event, Harvest stores the vote as `in-progress` and
publishes only its compact `(tenant, event, cast-vote ID)` identity—never ballot
contents—to `process_cast_vote` on the existing communication queue. The worker
reloads the vote and voter, serializes work per
event and voter, prepares and validates the MinIO template, and only then claims
the vote before contacting VoterView. Template, configuration, and Keycloak
preflight failures therefore leave the row `in-progress` for the recovery beat
to retry. The beat does not republish a row while another vote for that voter
is `indeterminate`.

The cast-vote states are:

- `valid`: accepted and eligible for receipts, statistics, and tallying.
- `discarded`: rejected or released; never included in those outputs.
- `in-progress`: safely stored but not yet claimed by the Datafix worker.
- `indeterminate`: claimed, but the external outcome cannot yet be proven.

`SetVoted` is not retried after an ambiguous response. A retry that receives
"already voted" cannot distinguish another channel from a successful first
request, so guessing could silently discard a legitimate ballot. Both
`in-progress` and `indeterminate` block tally extraction for their contest area.
An explicit `Success=false` rejection is terminal and discards the ballot.

Disabling an enabled Datafix voter is synchronous. Valid ballots are first
quarantined as `indeterminate`; after VoterView confirms `SetNotVoted` (including
the idempotent "has not voted" response), the Internet marker is cleared and
all of that voter's event ballots are discarded. If the Keycloak edit fails,
its outcome may be ambiguous, so the ballots remain quarantined instead of
being guessed back to `valid`. If VoterView is ambiguous, the voter also remains
disabled and the ballots remain indeterminate. A repeat save retries the
durably recorded pending release.

The SOAP templates remain public assets in MinIO:

- `voterview_setvoted.hbs`
- `voterview_setnotvoted.hbs`

They must be uploaded with the environment's other public assets. Template and
response bodies contain sensitive data and must not be copied into logs.

## Reconciling an indeterminate vote

Reconciliation is an operator decision, not an automatic retry:

1. Read the cast vote's `datafix_pending_operation` annotation and its signed
   electoral-log entries.
2. Verify the voter state directly in VoterView with the election authority.
3. For `set-not-voted`, retry the same disabled-voter save first. It is safe and
   converges on both success and "has not voted".
4. For `inbound-mark-voted` or `inbound-unmark-voted`, retry the corresponding
   authenticated Datafix API operation before using break-glass SQL.
5. For `set-voted`, record the authority's decision in the incident record. Set
   the ballot to `valid` only if VoterView confirms the Internet vote; otherwise
   set it to `discarded`. When accepting it, also restore the Keycloak
   `voted-channel=Internet` marker.

As a break-glass database operation, resolve one row with a compare-and-set so
a concurrent resolution cannot be overwritten:

```sql
BEGIN;

SELECT id, status, annotations ->> 'datafix_pending_operation' AS operation
FROM sequent_backend.cast_vote
WHERE id = '<cast-vote-uuid>'
FOR UPDATE;

UPDATE sequent_backend.cast_vote
SET status = '<valid-or-discarded>',
    annotations = COALESCE(annotations, '{}'::jsonb) - 'datafix_pending_operation',
    last_updated_at = NOW()
WHERE id = '<cast-vote-uuid>'
  AND status = 'indeterminate'
  AND annotations ->> 'datafix_pending_operation' = 'set-voted';

COMMIT;
```

The update must affect exactly one row. Zero rows means the state changed and
must be investigated again. Use the normal application retry for
`set-not-voted`; do not resolve only one of a voter's release rows manually.
