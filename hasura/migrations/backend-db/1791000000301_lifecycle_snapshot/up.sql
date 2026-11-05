-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

-- VOTE-LIFECYCLE §5b: what each publication recorded about the lifecycle
-- (policies, Open/Close voting rules, schedule), the published copy that
-- scheduled openings and closings are checked against. Written only by
-- publishing (and by a signed configuration approval, approval_request_id);
-- not tracked in Hasura, so no role can change it, and kept when the
-- publication row is deleted.
CREATE TABLE sequent_backend.lifecycle_snapshot (
    id uuid NOT NULL DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    ballot_publication_id uuid NOT NULL,
    election_id uuid,
    approval_request_id uuid,
    snapshot jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (id),
    CONSTRAINT lifecycle_snapshot_election_event_fkey FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE,
    CONSTRAINT lifecycle_snapshot_is_an_object CHECK (jsonb_typeof(snapshot) = 'object')
);

CREATE INDEX lifecycle_snapshot_newest
    ON sequent_backend.lifecycle_snapshot (tenant_id, election_event_id, created_at DESC);
