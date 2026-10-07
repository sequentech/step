-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

BEGIN;
-- Serializes concurrent initializations of the database.
SELECT pg_advisory_xact_lock(7307648119525449473);
-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
CREATE TABLE IF NOT EXISTS trellis_logs (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    size BIGINT NOT NULL DEFAULT 0 CHECK (size >= 0),
    root BYTEA NOT NULL DEFAULT '\xe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855' CHECK (octet_length(root) = 32)
);
-- Logs created before subtrees were stored gain the empty root here, which marks them
-- for a rebuild from their leaves before they are read or extended.
ALTER TABLE trellis_logs ADD COLUMN IF NOT EXISTS root BYTEA NOT NULL DEFAULT '\xe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855' CHECK (octet_length(root) = 32);
CREATE TABLE IF NOT EXISTS trellis_leaves (
    log_id BIGINT NOT NULL REFERENCES trellis_logs(id) ON DELETE CASCADE,
    leaf_index BIGINT NOT NULL CHECK (leaf_index >= 0),
    source_id BIGINT NOT NULL,
    hash BYTEA NOT NULL CHECK (octet_length(hash) = 32),
    PRIMARY KEY (log_id, leaf_index),
    UNIQUE (log_id, source_id)
);
-- Hash of the perfect subtree covering leaves [idx * 2^level, (idx + 1) * 2^level).
CREATE TABLE IF NOT EXISTS trellis_nodes (
    log_id BIGINT NOT NULL REFERENCES trellis_logs(id) ON DELETE CASCADE,
    level SMALLINT NOT NULL CHECK (level BETWEEN 1 AND 63),
    idx BIGINT NOT NULL CHECK (idx >= 0),
    hash BYTEA NOT NULL CHECK (octet_length(hash) = 32),
    PRIMARY KEY (log_id, level, idx)
);

CREATE TABLE IF NOT EXISTS electoral_log_boards (
    board_name TEXT PRIMARY KEY REFERENCES trellis_logs(name) ON DELETE CASCADE CHECK (board_name <> '')
);

CREATE TABLE IF NOT EXISTS electoral_log_messages (
    id BIGSERIAL PRIMARY KEY,
    board_name TEXT NOT NULL REFERENCES electoral_log_boards(board_name) ON DELETE CASCADE,
    delivery_id TEXT NOT NULL CHECK (delivery_id <> ''),
    created BIGINT NOT NULL,
    sender_pk TEXT NOT NULL,
    statement_timestamp BIGINT NOT NULL,
    statement_kind TEXT NOT NULL,
    message BYTEA NOT NULL,
    version TEXT NOT NULL,
    user_id TEXT,
    username TEXT,
    election_id TEXT,
    area_id TEXT,
    ballot_id TEXT,
    UNIQUE (board_name, delivery_id)
);
CREATE INDEX IF NOT EXISTS electoral_log_board_cursor ON electoral_log_messages (board_name, id);
CREATE INDEX IF NOT EXISTS electoral_log_cast_vote ON electoral_log_messages (board_name, statement_kind, election_id, id);
CREATE INDEX IF NOT EXISTS electoral_log_voter ON electoral_log_messages (board_name, user_id, ballot_id, id);
CREATE INDEX IF NOT EXISTS electoral_log_created ON electoral_log_messages (board_name, created, id);
-- Ballot box: cast votes stored in the electoral log. Tables are partitioned by
-- election event; an event's partitions are created with its board. A vote is
-- accepted when one statement has updated `ballot_box_voter`, stored the ballot
-- and queued it in `ballot_box_pending`; the sequencer appends its record to the
-- event's board later and removes it from the queue.
CREATE SEQUENCE IF NOT EXISTS ballot_box_seq;
CREATE TABLE IF NOT EXISTS ballot_box_ballot (
    election_event_id UUID NOT NULL,
    seq BIGINT NOT NULL DEFAULT nextval('ballot_box_seq'),
    id UUID NOT NULL DEFAULT gen_random_uuid(),
    ballot_id TEXT NOT NULL CHECK (ballot_id <> ''),
    election_id UUID NOT NULL,
    area_id UUID NOT NULL,
    voter_id TEXT NOT NULL CHECK (voter_id <> ''),
    format TEXT NOT NULL,
    content TEXT NOT NULL,
    voter_signature BYTEA,
    pseudonym_hash BYTEA NOT NULL CHECK (octet_length(pseudonym_hash) = 64),
    ballot_hash BYTEA NOT NULL CHECK (octet_length(ballot_hash) = 64),
    voting_channel TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('valid', 'pending', 'rejected')),
    voter_ip TEXT,
    voter_country TEXT,
    username TEXT,
    accepted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (election_event_id, seq),
    UNIQUE (election_event_id, ballot_id)
) PARTITION BY LIST (election_event_id);
-- A voter's ballots, for the voter's status and lookups.
CREATE INDEX IF NOT EXISTS ballot_box_ballot_voter ON ballot_box_ballot (election_event_id, voter_id);
-- The tally input of an area: each voter's latest ballot, in voter order, without
-- reading the rest of the event's ballots.
CREATE INDEX IF NOT EXISTS ballot_box_ballot_area
    ON ballot_box_ballot (election_event_id, election_id, area_id, voter_id, seq DESC);
-- Ballots whose outcome is pending, as Datafix events' ballots are until Datafix
-- answers: few at a time, looked up by ID and listed for review.
CREATE INDEX IF NOT EXISTS ballot_box_ballot_pending
    ON ballot_box_ballot (election_event_id, id) WHERE status = 'pending';
CREATE TABLE IF NOT EXISTS ballot_box_voter (
    election_event_id UUID NOT NULL,
    election_id UUID NOT NULL,
    voter_id TEXT NOT NULL,
    area_id UUID NOT NULL,
    votes INTEGER NOT NULL CHECK (votes >= 0),
    last_ballot_id TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (election_event_id, election_id, voter_id)
) PARTITION BY LIST (election_event_id);
CREATE TABLE IF NOT EXISTS ballot_box_pending (
    election_event_id UUID NOT NULL,
    seq BIGINT NOT NULL,
    PRIMARY KEY (election_event_id, seq)
);
-- Lease that lets one sequencer at a time work on an event.
CREATE TABLE IF NOT EXISTS ballot_box_sequencer (
    election_event_id UUID PRIMARY KEY,
    holder TEXT NOT NULL,
    lease_until TIMESTAMPTZ NOT NULL
);
COMMIT;
