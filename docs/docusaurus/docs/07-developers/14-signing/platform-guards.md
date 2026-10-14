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
| Publication generation, completion timestamp and immutable ballot-file root | Use the generation/publication workflow. Direct inserts and updates cannot forge or clear these values; equal values and unrelated labels or annotations are accepted. Generated or published publication identity and election membership remain fixed. |
| Initialization report session after private coverage is captured | Identity, tenant/event, selected elections/areas, tally type, keys ceremony, configuration and threshold cannot change through ordinary writes. Raw deletion is refused too. No-op writes, status/progress and unrelated annotations remain allowed; validated trusted cleanup remains available. |
| Generated or published ballot-style material | Direct changes to EML, signature, identity, publication, election or area are refused under either the old or new protected publication. Generate a new publication instead. Style tombstones and resurrection are protected too because they change country membership. Draft material and availability remain editable; unrelated metadata and equal values remain allowed. Retire protected styles through the authorized publication workflow. |

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

Publication status, immutable file annotation, ballot-generation and old-publication
retirement adapters mark their validated server transactions before changing protected
styles. Authorized event deletion also marks its transaction
before removing protected styles and their publication parents; full event import
already uses the marker. Ordinary Hasura saves cannot acquire it.

Untrusted style material writes lock their exact tenant/event publication parents in
stable order before checking generation or publication state. Parent locks use NOWAIT
because the style row may already be locked in the opposite order to a publishing
transaction. Contention returns SQLSTATE `55P03` with a retry message rather than
waiting in that inverse order. Publication approval owns the parent row while reading
its digest and preparing files, so a concurrent direct material edit cannot change
what the signed approval authorizes.

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

Published country membership is frozen configuration evidence. New lifecycle snapshots
and configuration signing subjects carry an optional `initialization_countries` map keyed
by Post. Capture the exact generated-publication style area IDs under the publication
lock, without intersecting mutable area/contest relationships at approval time. Absent
fields remain omitted from legacy canonical signing payloads. A missing map or missing
Post key derives conservatively from that exact retained publication's styles, including
retired rows; an explicit empty list means that publication had no countries for the Post.
An empty legacy fallback is evidence only when the exact retained generated publication
proves that target Post's empty membership. If neither exact styles nor that parent prove
membership, preserve the unknown map/key. A required Post under **Post and country**
refuses opening and initialization until a normal new publication supplies the evidence;
this uncertainty does not block closing or relax other initialized-state checks.

For country initialization, effective requirements are the union of current countries
and retained published countries. Editing live area/contest links, moving areas or retiring
styles cannot remove retained requirements. A new normal publication replaces the published
configuration only after its required approval. Report completion records only the countries
actually covered by its session. A whole-Post report cannot mark an uncovered required country
initialized; coverage together with prior completed country evidence must satisfy the effective
set before the Post's completion flag is set. A valid hash and document do not establish
country coverage on their own.

Creation-time report coverage is separate private evidence in
`sequent_backend.initialization_report_coverage`, keyed by tenant, event and tally session.
The creation transaction records actual generated-style coverage separately for each selected
Post, after applying the report's country filter. Explicit empty lists are evidence; another
Post's countries or the public session-wide `area_ids` union cannot substitute for a missing
Post key. Completion reads this immutable evidence and never reconstructs it from later
style or topology changes. The table is not tracked by Hasura; inserts require an authorized
server transaction and changed rows cannot be rewritten.

Once that evidence exists, the parent session's identity, tenant/event, `election_ids`,
`area_ids`, `tally_type`, `keys_ceremony_id`, `configuration` and `threshold` are frozen against
ordinary updates and deletion. Threshold is a decryption input. Progress/status changes,
no-op values and unrelated annotations remain allowed; `initialization_area_ids` annotations
do not select execution or establish coverage. Authorized event cleanup marks its transaction
before removing children; scoped session and event foreign keys cascade private evidence
during authorized cleanup.

An unfinished legacy initialization session without private coverage requires a new report
session and its normal authorization; re-running it cannot guess the old covered countries.
Previously completed, recorded initializations are returned idempotently before this new
proof requirement is checked and retain their existing history.

Enrollment schedule saves, CSV imports and accepted timezone recomputations finish their
local validation and audit writes before installing a nonempty denial marker in the
realm, while holding the event scheduling lock. They then commit and refresh enrollment
windows from committed database state under a fresh lock. Validation rollback leaves
existing windows unchanged; an ambiguous commit or synchronization failure leaves
registration refused until a successful refresh. Retry the operation after correcting
the reported failure. A successful remote update is never treated as a database commit.
