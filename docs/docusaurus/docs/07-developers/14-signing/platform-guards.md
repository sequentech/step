<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Platform guards: what they cover

PostgreSQL triggers protect voting state and lockdown when Hasura roles can save
whole election records. They apply to direct SQL too. They preserve other JSON
settings and accept reserialization of defaults and equal timestamp instants.

| Write | Rule |
| --- | --- |
| Election or event channel voting status | Use the voting action or scheduler state machine. Direct updates cannot change any of the four channels. |
| Channel period dates | Use the same state machine; direct updates cannot move the close used for grace votes. |
| New election or event | Start with voting not started on every channel and no period dates. |
| Event lockdown | Use scheduled start/end lockdown. Direct updates and non-default inserts are refused. |
| Election grace policy or duration | Configurable before voting starts; protected after any channel starts or has recorded period dates. |
| Election identity, tenant or parent event | Ordinary changes are refused after voting starts, initialization evidence exists, a signed voting boundary is retained, or an applicable Post/event configuration has been published. This includes published initialization policies without voting schedules. A new Post in an unpublished event without that evidence can move; authorized server transactions retain the existing foreign-key checks and refresh signed boundaries. |

Server state-machine paths mark their transaction with
`windmill::postgres::trusted_write` (`SET LOCAL sequent.trusted_write = 'on'`).
The mark ends at transaction completion. Generic database update helpers must
not set it: otherwise stale publication, initialization or form writes could
restore obsolete voting state. Event import resets every channel and its dates
before insertion; it marks the transaction to preserve exported lockdown.
The marker authorizes subsequent writes in the transaction, so call it only
in audited server paths after authorization and validation. It is not a defence
against a database administrator, raw SQL execution permission or a compromised
backend. Hasura roles must never expose SQL execution or this marker.

A stale form that restores an old protected value is refused with SQLSTATE
`42501`. Reload and repeat edits against the current record. Admin notifications
explain which action to use. The lockdown selector is read-only and preserves
its stored value on save.

These triggers do not authorize scheduled-event CRUD or sign lockdown changes.
A permitted direct insertion of `END_LOCKDOWN_PERIOD` can still request a
scheduled unlock. Signing-rule changes after an unlock use the signing feature's
live rule checks. Archiving or disabling voting channels can also stop casting;
these settings are not covered by the voting-state triggers. Lifecycle-window
fields are not guarded here: they require the scheduler integration to mark its
writes if casting begins to depend on them. These boundaries must be considered
when assigning configuration and scheduling permissions.

The focused `windmill/tests/postgres_trusted_write.rs` suite applies real
migrations in an isolated fixture database. It covers rejected direct writes,
allowed ordinary saves, default inserts, grace configuration, trusted transaction
lifetime, event-wide state writes and lockdown audit entries. Run it with the
checkout's own Cargo target and a disposable PostgreSQL server, following the
[build guide](../03-development-environment/build-and-test.md).

Initialization completion is also a trusted write: ordinary inserts and updates cannot
set or clear `initialization_report_generated`. A completed report must have a nonempty
hash and a PDF or HTML document before initialization evidence is recorded. New
publications retain each Post's initialization report policy; removing a live requirement
cannot bypass a retained requirement, including the other required Posts under EVENT scope.
Older snapshots and signed subjects lack this policy map and cannot reconstruct a previous
value, so they retain their established live-policy behavior. The completion flag remains
protected for those events too.

Enrollment schedule saves, CSV imports and accepted timezone recomputations finish their
local validation and audit writes before installing a nonempty denial marker in the
realm, while holding the event scheduling lock. They then commit and refresh enrollment
windows from committed database state under a fresh lock. Validation rollback leaves
existing windows unchanged; an ambiguous commit or synchronization failure leaves
registration refused until a successful refresh. Retry the operation after correcting
the reported failure. A successful remote update is never treated as a database commit.
