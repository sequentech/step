-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

-- VOTE-LIFECYCLE §5a: each scheduled opening or closing that ran at a Post,
-- once. A covered transition that already ran there with the same
-- fingerprint isn't covered again (a re-armed or re-inserted row needs
-- signatures). Written only by the scheduler; not tracked in Hasura.
CREATE TABLE sequent_backend.lifecycle_fired (
    id uuid NOT NULL DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    scheduled_event_id uuid NOT NULL,
    election_id uuid NOT NULL,
    fingerprint text NOT NULL,
    fired_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (id),
    CONSTRAINT lifecycle_fired_election_event_fkey FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE,
    CONSTRAINT lifecycle_fired_fingerprint_sha256_hex CHECK (fingerprint ~ '^[0-9a-f]{64}$')
);

CREATE INDEX lifecycle_fired_of_event
    ON sequent_backend.lifecycle_fired (tenant_id, election_event_id, scheduled_event_id);
