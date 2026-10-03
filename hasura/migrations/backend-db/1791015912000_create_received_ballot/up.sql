-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Ballots the ballot box has stored and signed at review, before they are cast.
CREATE TABLE sequent_backend.received_ballot (
    id uuid NOT NULL DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    election_id uuid NOT NULL,
    area_id uuid NOT NULL,
    voter_id_string text NOT NULL,
    ballot_id text NOT NULL,
    ballot_hash text NOT NULL,
    content text NOT NULL,
    voter_signing_pk text NOT NULL,
    voter_ballot_signature text NOT NULL,
    received_at timestamptz NOT NULL,
    key_id text NOT NULL,
    received_signature text NOT NULL,
    status text NOT NULL DEFAULT 'received',
    cast_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    last_updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (id, tenant_id, election_event_id),
    CONSTRAINT received_ballot_status_check
        CHECK (status IN ('received', 'cast', 'audited')),
    -- A Ballot ID names one ballot in its election event.
    CONSTRAINT received_ballot_ballot_id_key
        UNIQUE (tenant_id, election_event_id, ballot_id),
    -- Receiving the same ballot again answers with the stored receipt.
    CONSTRAINT received_ballot_voter_ballot_key
        UNIQUE (tenant_id, election_event_id, election_id, voter_id_string, ballot_hash)
);

-- Encrypted ballots do not compress, as in cast_vote.
ALTER TABLE sequent_backend.received_ballot ALTER COLUMN content SET STORAGE EXTERNAL;

ALTER TABLE sequent_backend.cast_vote ADD COLUMN received_ballot_id uuid;
