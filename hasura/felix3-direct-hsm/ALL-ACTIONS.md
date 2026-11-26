<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Optional direct-approval demo continuation

This source does not enable the extended demo. It uses the existing exact
installation and database-free PKCS#11 signer. Harvest authenticates the
normal Portal account, rechecks the action role, Post or trustee, and frozen
request and rule; the private signer receives a bounded request from
Harvest, not a browser or a service database account.

Before any future review of activation:

1. Pin the matched Step Harvest and Windmill workers, Admin Portal/optional
   Hasura metadata, and Beyond private signer builds. Do not enable the new
   marker while any older server or worker can execute the same event.
   Verify that all selected runtime database users can read the narrow
   action-scope helper with the existing sequent_os_approve authority.
2. Apply the optional 1794700002295_direct_action_evidence migration only
   in the isolated approved demo lineage, after the original optional
   migrations. It does not install or enable a marker.
3. On lineages using manual identity verification, apply the matching standard
   catalog migration 1794700002320 and optional migration 1794700002322 after
   2302, before admitting the new clients. This registers one additional action
   without changing any rule, enabling direct mode or granting account privileges.
   Validate the imported event, anchor Post, publication and its required rules. Reconcile pending legacy requests before any
   operator enables the new marker. The exact existing V1 Initialize on
   the anchor is not reclassified.
   Where this lineage builds MIRU packages, apply the standard migration
   1794700002340 and then optional migration 1794700002342. They let the
   central registry classify, and so withhold, the output of the exact
   anchored Post or the single Post of a device-root event. They add no
   tenant, signer, proof or release authority.
4. Verify the genuine isolated actor-key enrollment, separate actors meeting
   each threshold, expiration and replay defenses, the new exact publication
   after a configuration approval, and all deferred effects and trustee
   consumptions. Keep report/artifact and transport evidence separate.
5. If operator-only staging must roll back before any direct action
   activity, use the drained maintenance procedure below. An installation
   used by the action workflow is not reversible; preserve the marker and
   recorded authority and roll forward with matching HSM-only binaries.

A detached approval covers the signed payload, including the immutable
reviewed document digest when the action has a document. It is not a
PAdES/CMS/EML certification. Existing independently valid PDF revisions use
the stock release checks. A MIRU package without independently trusted
member document signatures is not built or transmitted from raw actor
approvals. Do not use the demo to claim official certification or delivery.

## More than one event, without an event allowlist

Migration `1794700002302_direct_multi_event_approval` retains the installed
device and tenant, its first event's original V1 proofs, and the independent
V2 action marker. It derives other eligible events and whole Post scopes
from the ordinary signed-package import and ballot generation, not a
tenant/event choice or copied installer pin. A certificate is not a fallback
when the mounted signer or exact saved scope is unavailable.

1. Match the dedicated Portal, Harvest, Windmill and Beyond signer binaries
   before applying verification or enabling the existing installation's
   action marker. Drain older application processes from affected requests.
   The signer continues to hold the same token keys and has no database
   account. Do not grant the receipt writer to browser roles or application
   approval groups; it is the trusted stock generation process's callback.
2. For another eligible Post, configure an ordinary required rule at
   **Signatures → Protected actions → Approve a configuration version**.
   Then go to the Post's **Publish** page, **Regenerate** (or **Generate
   Publication**), and **Publish Changes**. The ordinary authorization checks
   still apply. The required signers review the exact generated configuration
   and its files; until that effect completes, a previous published ballot or
   signed import alone does not authorize the next Initialize.
3. Start **Initialize Voting** on that exact newly authorized publication.
   A waiting legacy request is not converted even if nobody has signed it.
   It remains readable; use the normal cancellation, supersession, or expiry
   path before starting a new request. Existing V1 evidence stays readable
   and is not counted as V2 evidence.
4. Actions already scoped to a Post keep that exact Post. A whole-event
   operation only selects a Post when that choice is unambiguous. Multiple
   Post selections never become a silent “first Post” proof. When an
   authorized next publication is missing, the scoped forms fail closed.
5. Confirm both exact event paths and the matched artifacts in an isolated
   fixture before rollout. Do not remove these migrations or restart older
   approval code over new requests, generation, reservations, or proof.

### Roll back unused staging only

This is a migration-owner maintenance operation, not an approval-mode switch.
Before traffic is admitted, drain Harvest, Windmill, the scheduler and all
other database writers. To back out the unused staging marker, run the
`2322` (if applied), `2302` and then `2295` down SQL in one explicit read-committed transaction
with stop-on-error and a bounded lock timeout. Leave the original installation,
R2 V1 evidence and prior migrations in place. The transaction refuses dynamic
request or generation history, any V2 reservation or proof, activity or
ambiguous history on the anchor event, or a busy write barrier. If it refuses,
leave the existing evidence and schema intact and roll forward; do not
force the removal or revert to certificate approval.
