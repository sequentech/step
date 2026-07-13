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

- `valid`: accepted and eligible for statistics and tallying.
- `discarded`: rejected or released; excluded from statistics and tallying.
- `in-progress`: safely stored but not yet claimed by the Datafix worker.
- `indeterminate`: claimed, but the external outcome cannot yet be proven.

The voter portal confirms the ballot immediately after Harvest stores it. It
does not wait for or display the VoterView result; these states support
back-office reconciliation, revote eligibility, reporting, and tally safety.
The voter-facing locator and receipt prove that Sequent stored the submitted
ballot and therefore remain available regardless of its later Datafix status.

`SetVoted` is not retried after an ambiguous response. A retry that receives
"already voted" cannot distinguish another channel from a successful first
request, so guessing could silently discard a legitimate ballot. Both
`in-progress` and `indeterminate` block tally extraction for their contest area.
An explicit `Success=false` rejection is terminal and discards the ballot.

Disabling an enabled Datafix voter is synchronous. Valid ballots are first
quarantined as `indeterminate`; after VoterView confirms `SetNotVoted` (including
the idempotent "has not voted" response), the Internet marker is cleared and
all of that voter's event ballots are discarded. If the Keycloak edit fails,
its outcome may be ambiguous, so the pre-dispatch quarantine is restored to
`valid` to avoid silently losing the ballots. If Keycloak did apply the update,
the pending release marker makes a repeated disabled-voter save retry the
release. If VoterView is ambiguous, the voter remains disabled and the ballots
remain indeterminate. A repeat save retries the durably recorded pending
release.

The SOAP templates remain public assets in MinIO:

- `voterview_setvoted.hbs`
- `voterview_setnotvoted.hbs`

They must be uploaded with the environment's other public assets. Template and
response bodies contain sensitive data and must not be copied into logs.

Because the templates live in MinIO, their SOAP structure can be updated without
redeploying Harvest. To keep that flexibility, a rendered request is checked only
for the invariants a correct template cannot violate—that it is well-formed XML
and that every injected value (voter id, credentials, timestamp) survives
rendering as escaped text—never for specific element names, so a VoterView-side
change stays a template edit. For provenance, the SHA-256 of the template file
that produced each outbound request is recorded in that request's electoral-log
entry, so the exact template version behind any request can be audited after the
fact. The hash is recorded only; it is never used to gate a request.

## End-to-end walkthrough: releasing a voter who already voted online

This example follows one voter, Nadia, through the whole lifecycle and names the
party that calls each SOAP or Harvest endpoint.

**She votes online.** Nadia submits her ballot in the Voting Portal, which calls
the Hasura `insert_cast_vote` action and reaches **Harvest `/insert-cast-vote`**.
Because this is a Datafix event, Harvest stores the ballot as `in-progress` and
queues `process_cast_vote`, but confirms her receipt immediately without waiting
for the external result.

**The online vote is recorded.** A Windmill worker picks up the job, locks the
voter, validates the template, claims the vote, and calls out:

> **Windmill → VoterView (SOAP) `SetVoted`** — "Nadia voted, via Internet."

On success her ballot moves `in-progress → valid` and becomes countable.

**She shows up to vote in person.** She cannot, while the system believes she
already voted online, so a back-office operator disables her in the Admin
Portal. That calls the Hasura `edit_user` action and reaches **Harvest
`/edit-user`**, which is synchronous—the operator needs the outcome before
letting her vote in person. Harvest recognizes a *disable* transition on a
Datafix voter who holds a `valid` vote and runs a reversible sequence:

1. **`quarantine_valid_cast_votes`** flips her `valid` ballots to
   `indeterminate` (tagged `set-not-voted`) and returns their IDs. They stop
   counting immediately—before any external call—and the saved IDs make the step
   undoable.
2. > **Harvest → VoterView (SOAP) `SetNotVoted`** — "Nadia has not voted online;
   > release her." The idempotent "has not voted" reply also counts as success.
3. Branch on the results:
   - **Converged** → clear her Internet marker, disable the account in Keycloak,
     then **`finalize_voter_release`** discards all her event ballots. She is now
     free to vote in person.
   - **Keycloak edit fails** → **`restore_quarantined_cast_votes`** flips exactly
     those saved IDs back to `valid`, so nothing is silently lost.
   - **VoterView is ambiguous** → the ballots stay `indeterminate` and a durable
     pending-release marker is recorded (`mark_voter_release_pending`). The
     operator simply saves again; the marker makes the repeat retry the release
     rather than re-quarantine.

The same quarantine step is reached from the opposite direction when the
external Datafix system calls **Harvest `/api/datafix/*`** to report that a voter
voted through another channel; that inbound path quarantines (tagged
`inbound-mark-voted` / `inbound-unmark-voted`) with the same discard-or-restore
safety branches. In both directions the rule is identical: stop counting first,
act externally second, and keep enough state to undo or converge.

## Reconciling an indeterminate vote

Reconciliation is an operator decision, not an automatic retry:

1. Read the cast vote's `datafix_pending_operation` annotation and its signed
   electoral-log entries.
2. Verify the voter state directly in VoterView with the election authority.
3. For `set-not-voted`, retry the same disabled-voter save first. It is safe and
   converges on both success and "has not voted".
4. For `inbound-mark-voted` or `inbound-unmark-voted`, retry the corresponding
   authenticated Datafix API operation before using break-glass SQL.
5. For `set-voted`, the outbound `SetVoted` returned an ambiguous result, so the
   ballot is stuck `indeterminate` without proof it registered. Using the
   VoterView state verified in step 2, record the authority's decision in the
   incident record. Set the ballot to `valid` only if VoterView's records show
   the voter's Internet vote was registered; otherwise set it to `discarded`.
   When accepting it, also restore the Keycloak `voted-channel=Internet` marker,
   which the ambiguous path never set.

As a break-glass database operation, resolve one row with a compare-and-set so
a concurrent resolution cannot be overwritten:

```sql
BEGIN;

SELECT id, status, annotations ->> 'datafix_pending_operation' AS operation
FROM sequent_backend.cast_vote
WHERE id = '<cast-vote-uuid>'
  AND tenant_id = '<tenant-uuid>'
  AND election_event_id = '<election-event-uuid>'
FOR UPDATE;

UPDATE sequent_backend.cast_vote
SET status = '<valid-or-discarded>',
    annotations = COALESCE(annotations, '{}'::jsonb) - 'datafix_pending_operation',
    last_updated_at = NOW()
WHERE id = '<cast-vote-uuid>'
  AND tenant_id = '<tenant-uuid>'
  AND election_event_id = '<election-event-uuid>'
  AND status = 'indeterminate'
  AND annotations ->> 'datafix_pending_operation' = 'set-voted';

COMMIT;
```

The composite key must identify exactly one row, and the update must affect that
row. Zero rows means the state changed and must be investigated again. Use the
normal application retry for `set-not-voted`; do not resolve only one of a
voter's release rows manually.
