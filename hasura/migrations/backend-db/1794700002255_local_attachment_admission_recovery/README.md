<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Local Security Officer attachment admission recovery

Apply 2255 transactionally after the original, unchanged 2250 migration and
before any later local attachment overlay. Keep the original trust and role
installation. This migration preserves existing decisions and the existing
officer and registry EXECUTE grants. It does not grant any runtime role access
to the decision table. Use a short maintenance window with no attachment
request running; if the ledger is busy the NOWAIT lock makes the migration
fail before any changes. Retry later, not by editing the old migration history.

The migration backfills two immutable snapshots from each original decision
under the same lock. It does not discard or reset any certificate, request,
writer, trust observation or lease. A prerelease 2250 with the recovery fields
already inlined is not the stock 2250 layout and is rejected before any changes.
Reconcile such an installation with a verified backup; do not delete decisions
or mark this migration applied to skip the check.

An unsealed admission can be replaced only after PostgreSQL confirms that
the previous writer's exact full transaction ID aborted or committed without
sealing. Lease expiry, PID reuse and unavailable XID history are not evidence
of termination. The new login/PID must hold its own live transaction-ID lock.
An identical, still-valid request by the same writer is read-only and cannot
extend the deadline. A new writer always needs fresh authorization and proof.

The attempt, officer, registry login, installation, tenant, event, Post,
publication, nominee, enrollment, CSR, SPKI and approved issuer/root stay
fixed. A current signed CRL, its observation, or a genuinely reissued leaf
for the same key may change. Only the request ID and payload digest may move
within that scope, and only when the installed generation resolver independently
authorizes that exact current action. 2255 alone does not authorize a
successor request. A sealed decision is never rebound; recovering its receipt
requires the exact settled request, pins, trust stamp and registry evidence.

A changed sealed trust observation, a CA rotation, or indeterminate transaction
history requires operator reconciliation. The rollback is deliberately blocked
while any decision exists; it never deletes evidence or reintroduces the old
recovery rule over existing decisions. A clean rollback restores the two original
2250 function bodies and their UPDATE permissions without replacing service ACLs.

Run the combined source check from the Step root:

    python3 -I -B packages/harvest/tests/local_initialize_attachment_source.py -v
