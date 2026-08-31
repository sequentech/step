---
id: support_materials
title: Support Materials
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Support Materials

Support Materials are event-level documents (guides, sample ballots, instructional videos) that voters can review before voting. The Support Materials Policy controls whether they are shown at all, and whether reading them is a precondition for voting.

## Policy

Configure the policy in the Support Materials accordion on the Election Event Data tab:

- `Off` hides Support Materials entirely in the Voting Portal and does not gate voting.
- `Optional` shows Support Materials as a generic link from the Ballot list; voting is never blocked.
- `Mandatory for Voting` requires each voter to open every listed Support Material and acknowledge having read them before any ballot's Start Voting control is enabled.

The policy applies to the whole Election Event; there is no per-election override. Election events created before this policy existed carry only the legacy `presentation.materials.activated` boolean — `Optional` when `true`, `Off` when `false` or absent — until an administrator sets the policy explicitly.

## Voter experience under Mandatory for Voting

When the policy is `Mandatory for Voting` and a voter has not yet acknowledged the materials for this Election Event, the Ballot list shows an instruction banner ("You must read the Support Materials before you can vote.") and disables Start Voting on every ballot. The Support Materials button remains available so the voter can open it.

On the Support Materials screen, each material must be opened at least once via Preview; a material is marked viewed the moment its preview dialog is closed. The "I have read the Support Materials" checkbox stays disabled until every listed material has been viewed, and Continue stays disabled until the checkbox is checked. Choosing Continue records the acknowledgment and returns the voter to the Ballot list with Start Voting enabled; choosing Back to ballot list leaves the Ballot list gated, since nothing is recorded until Continue succeeds.

Once acknowledged, the gate is skipped on later visits for that Election Event — the Support Materials button stays available to reopen the documents, but Start Voting no longer requires it.

## Acknowledgment storage

Acknowledgment is recorded as a Keycloak user attribute (`support-materials-acknowledged`) on the voter's account, holding the list of Support Material document ids they confirmed reading. Since each Election Event has its own Keycloak realm, the attribute is inherently scoped per voter per Election Event — the same pattern used for `voted-channel`. Reopening materials after acknowledging does not clear it; there is currently no bulk-reset action for administrators.

The Election Event Voters list shows a "Support Materials Viewed" column — a plain boolean read from that same attribute — when the policy is `Mandatory for Voting`.

## Requests and permissions

The Voting Portal reads and writes acknowledgment through two Hasura Actions backed by Harvest, both authenticated with the voter's own bearer token (`role: user`, `forward_client_headers: true`):

- `get_support_materials_acknowledgment(election_event_id)` returns the document ids the calling voter has already acknowledged.
- `acknowledge_support_materials(election_event_id, document_ids)` records acknowledgment for the calling voter, after checking the Election Event's policy is not `Off`.

Both actions derive the voter's identity from their own JWT claims — a voter can only ever read or write their own acknowledgment.

Both actions also require the voter's Keycloak session to carry a dedicated `ack-support-materials` realm role, checked the same way ballot casting checks the voter's `user` role. This role must exist in the Election Event's own realm (`tenant-<tenant_id>-event-<election_event_id>`) and be granted to the `voter` group — without it, every voter is rejected regardless of the Support Materials Policy.

### Adding the role in the Keycloak Console

1. Open the Keycloak Admin Console and switch to the Election Event's realm (`tenant-<tenant_id>-event-<election_event_id>`).
2. Under **Realm roles**, click **Create role**, set **Role name** to `ack-support-materials`, and save.
3. Under **Groups**, open **voter**, go to its **Role mapping** tab, click **Assign role**, filter by realm roles, select `ack-support-materials`, and click **Assign**.
4. Voters who are already signed in keep whatever role set was in their current token — they need to log out and back in (or wait for their token to refresh) before the new role takes effect.
