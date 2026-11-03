-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

-- Prototype of the accept path of the log-based ballot box: one database per
-- tenant, tables partitioned by election event. A vote is accepted when one
-- statement has stored the ballot and updated the voter state; the sequencer
-- adds accepted ballots to the Merkle log later.

CREATE TABLE ballot (
    election_event_id uuid        NOT NULL,
    seq               bigint      GENERATED ALWAYS AS IDENTITY,
    ballot_id         bytea       NOT NULL,
    election_id       uuid        NOT NULL,
    voter             bytea       NOT NULL,
    area_id           uuid,
    content           bytea       NOT NULL,
    accepted_at       timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (election_event_id, seq),
    UNIQUE (election_event_id, ballot_id)
) PARTITION BY LIST (election_event_id);

CREATE TABLE voter_state (
    election_event_id uuid        NOT NULL,
    election_id       uuid        NOT NULL,
    voter             bytea       NOT NULL,
    votes             integer     NOT NULL,
    last_ballot_id    bytea       NOT NULL,
    updated_at        timestamptz NOT NULL,
    PRIMARY KEY (election_event_id, election_id, voter)
) PARTITION BY LIST (election_event_id);

CREATE TABLE ballot_event1 PARTITION OF ballot
    FOR VALUES IN ('00000000-0000-4000-8000-000000000001');
CREATE TABLE voter_state_event1 PARTITION OF voter_state
    FOR VALUES IN ('00000000-0000-4000-8000-000000000001');

-- Ciphertexts do not compress: store them as they are, as real ballots would be.
ALTER TABLE ballot_event1 ALTER COLUMN content SET STORAGE EXTERNAL;
