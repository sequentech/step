-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

BEGIN;
-- Serializes concurrent initializations of the database.
SELECT pg_advisory_xact_lock(7307648119525449473);
-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-- `id` keys a log's rows in this database. `uid` identifies the log wherever it is
-- stored: checkpoints and leaf commitments name it, so a log copied to another
-- database keeps its checkpoints. A sealed log takes no more leaves.
CREATE TABLE IF NOT EXISTS trellis_logs (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    uid UUID NOT NULL DEFAULT gen_random_uuid(),
    size BIGINT NOT NULL DEFAULT 0 CHECK (size >= 0),
    root BYTEA NOT NULL DEFAULT '\xe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855' CHECK (octet_length(root) = 32),
    sealed_at TIMESTAMPTZ
);
-- Logs created before subtrees were stored gain the empty root here, which marks them
-- for a rebuild from their leaves before they are read or extended.
ALTER TABLE trellis_logs ADD COLUMN IF NOT EXISTS root BYTEA NOT NULL DEFAULT '\xe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855' CHECK (octet_length(root) = 32);
ALTER TABLE trellis_logs ADD COLUMN IF NOT EXISTS uid UUID NOT NULL DEFAULT gen_random_uuid();
ALTER TABLE trellis_logs ADD COLUMN IF NOT EXISTS sealed_at TIMESTAMPTZ;
CREATE UNIQUE INDEX IF NOT EXISTS trellis_logs_uid ON trellis_logs (uid);
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
-- election event; an event's partitions are created with its board, in the event's
-- database. A vote is
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
-- Ballots the ballot box received at review and signed a receipt for, under the event's
-- signed-by-ballot-box receipts policy. `ballot_id` is the receipt's short Ballot ID and
-- `ballot_hash` the ballot's hash as the voter sent it, which is the `ballot_id` of the
-- ballot once cast. Casting a received ballot stores it in `ballot_box_ballot` and marks
-- it cast in the same statement.
CREATE TABLE IF NOT EXISTS ballot_box_received (
    election_event_id UUID NOT NULL,
    id UUID NOT NULL DEFAULT gen_random_uuid(),
    ballot_id TEXT NOT NULL CHECK (ballot_id <> ''),
    election_id UUID NOT NULL,
    area_id UUID NOT NULL,
    voter_id TEXT NOT NULL CHECK (voter_id <> ''),
    ballot_hash TEXT NOT NULL CHECK (ballot_hash <> ''),
    content TEXT NOT NULL,
    voter_signing_pk TEXT NOT NULL,
    voter_ballot_signature TEXT NOT NULL,
    received_at TIMESTAMPTZ NOT NULL,
    key_id TEXT NOT NULL,
    received_signature TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'received' CHECK (status IN ('received', 'cast', 'audited')),
    cast_at TIMESTAMPTZ,
    cast_signature TEXT,
    cast_receipt_signature TEXT,
    PRIMARY KEY (election_event_id, id),
    -- A Ballot ID names one ballot in its election event.
    UNIQUE (election_event_id, ballot_id),
    -- Receiving the same ballot again answers with the stored receipt.
    UNIQUE (election_event_id, election_id, voter_id, ballot_hash),
    CHECK (
        (status = 'cast') = (cast_at IS NOT NULL)
        AND (cast_signature IS NULL) = (cast_receipt_signature IS NULL)
        AND (cast_signature IS NULL OR status = 'cast')
    )
) PARTITION BY LIST (election_event_id);
-- Seals of ballot boxes, the ballots of one election's area. A box with a seal, sealed
-- or failed, takes no more writes. A seal is stored with its record on the event's board
-- in one transaction, and never changes.
CREATE TABLE IF NOT EXISTS ballot_box_seal (
    election_event_id UUID NOT NULL,
    election_id UUID NOT NULL,
    area_id UUID NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('sealed', 'failed')),
    -- SHA-512 of the manifest.
    seal_hash BYTEA CHECK (octet_length(seal_hash) = 64),
    manifest BYTEA,
    ballots_in_box BIGINT CHECK (ballots_in_box >= 0),
    ballots_counted BIGINT CHECK (ballots_counted >= 0),
    failure_reason TEXT,
    -- The signed `BallotBoxSealed` or `BallotBoxSealFailed` message, as the board stores it.
    signed_message BYTEA NOT NULL,
    log_delivery_id TEXT NOT NULL CHECK (log_delivery_id <> ''),
    log_uid UUID NOT NULL,
    log_leaf_index BIGINT NOT NULL CHECK (log_leaf_index >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (election_event_id, election_id, area_id),
    CHECK (CASE status
        WHEN 'sealed' THEN seal_hash IS NOT NULL AND manifest IS NOT NULL
            AND ballots_in_box IS NOT NULL AND ballots_counted IS NOT NULL
            AND failure_reason IS NULL
        ELSE failure_reason IS NOT NULL AND seal_hash IS NULL AND manifest IS NULL
            AND ballots_in_box IS NULL AND ballots_counted IS NULL
    END)
);
CREATE OR REPLACE FUNCTION ballot_box_seal_permanence() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'ballot_box_seal_permanent'
        USING ERRCODE = '42501',
              DETAIL = 'Ballot box seals can''t be changed, deleted or truncated.';
END;
$$;
-- The advisory-lock key of a ballot box's seal. Writes of the box's ballots take it
-- shared, the sealer exclusively.
CREATE OR REPLACE FUNCTION ballot_box_seal_lock_key(event UUID, election UUID, area UUID)
RETURNS TEXT LANGUAGE sql IMMUTABLE AS $$
    SELECT 'ballot-box-seal:' || event::text || ':' || election::text || ':' || area::text;
$$;
-- Wait for a seal of the box in progress, then refuse the write if the box has a seal.
-- Under READ COMMITTED the check reads a snapshot taken after the wait.
CREATE OR REPLACE FUNCTION ballot_box_seal_check(event UUID, election UUID, area UUID)
RETURNS void LANGUAGE plpgsql AS $$
BEGIN
    PERFORM pg_advisory_xact_lock_shared(
        hashtextextended(ballot_box_seal_lock_key(event, election, area), 0));
    IF EXISTS (
        SELECT 1 FROM ballot_box_seal
        WHERE election_event_id = event AND election_id = election AND area_id = area
    ) THEN
        RAISE EXCEPTION 'ballot_box_sealed'
            USING ERRCODE = '42501',
                  DETAIL = format('The ballot box of election %s, area %s is sealed.',
                                  election, area);
    END IF;
END;
$$;
-- A ballot's write is checked against the box it leaves and the one it enters, in key
-- order. Another isolation level than READ COMMITTED could check a snapshot that misses
-- a seal committed while the write waited, so it is refused.
CREATE OR REPLACE FUNCTION ballot_box_seal_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF current_setting('transaction_isolation') <> 'read committed' THEN
        RAISE EXCEPTION 'ballot_box_seal_requires_read_committed'
            USING ERRCODE = '42501',
                  DETAIL = format('Ballot box writes must run at READ COMMITTED, not %s.',
                                  current_setting('transaction_isolation'));
    END IF;
    IF TG_OP = 'INSERT' THEN
        PERFORM ballot_box_seal_check(NEW.election_event_id, NEW.election_id, NEW.area_id);
        RETURN NEW;
    END IF;
    IF TG_OP = 'DELETE' THEN
        PERFORM ballot_box_seal_check(OLD.election_event_id, OLD.election_id, OLD.area_id);
        RETURN OLD;
    END IF;
    IF (OLD.election_event_id, OLD.election_id, OLD.area_id)
       = (NEW.election_event_id, NEW.election_id, NEW.area_id) THEN
        PERFORM ballot_box_seal_check(NEW.election_event_id, NEW.election_id, NEW.area_id);
    ELSIF ballot_box_seal_lock_key(OLD.election_event_id, OLD.election_id, OLD.area_id)
          < ballot_box_seal_lock_key(NEW.election_event_id, NEW.election_id, NEW.area_id) THEN
        PERFORM ballot_box_seal_check(OLD.election_event_id, OLD.election_id, OLD.area_id);
        PERFORM ballot_box_seal_check(NEW.election_event_id, NEW.election_id, NEW.area_id);
    ELSE
        PERFORM ballot_box_seal_check(NEW.election_event_id, NEW.election_id, NEW.area_id);
        PERFORM ballot_box_seal_check(OLD.election_event_id, OLD.election_id, OLD.area_id);
    END IF;
    RETURN NEW;
END;
$$;
-- TRUNCATE skips row triggers. The guard of the partitioned table refuses it while any
-- box is sealed; that of an event's partition, created with it, takes the event's ID.
CREATE OR REPLACE FUNCTION ballot_box_seal_guard_truncate() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF current_setting('transaction_isolation') <> 'read committed' THEN
        RAISE EXCEPTION 'ballot_box_seal_requires_read_committed'
            USING ERRCODE = '42501',
                  DETAIL = format('Ballot box writes must run at READ COMMITTED, not %s.',
                                  current_setting('transaction_isolation'));
    END IF;
    IF EXISTS (
        SELECT 1 FROM ballot_box_seal
        WHERE TG_NARGS = 0 OR election_event_id = TG_ARGV[0]::uuid
    ) THEN
        RAISE EXCEPTION 'ballot_box_sealed'
            USING ERRCODE = '42501',
                  DETAIL = format('%s holds sealed ballot boxes and can''t be truncated.',
                                  TG_TABLE_NAME);
    END IF;
    RETURN NULL;
END;
$$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = 'ballot_box_seal'::regclass
                   AND tgname = 'ballot_box_seal_permanence') THEN
        CREATE TRIGGER ballot_box_seal_permanence BEFORE UPDATE OR DELETE ON ballot_box_seal
            FOR EACH ROW EXECUTE FUNCTION ballot_box_seal_permanence();
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = 'ballot_box_seal'::regclass
                   AND tgname = 'ballot_box_seal_permanence_truncate') THEN
        CREATE TRIGGER ballot_box_seal_permanence_truncate BEFORE TRUNCATE ON ballot_box_seal
            FOR EACH STATEMENT EXECUTE FUNCTION ballot_box_seal_permanence();
    END IF;
    -- A row trigger of the partitioned table is cloned to its partitions.
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = 'ballot_box_ballot'::regclass
                   AND tgname = 'ballot_box_seal_guard') THEN
        CREATE TRIGGER ballot_box_seal_guard BEFORE INSERT OR UPDATE OR DELETE
            ON ballot_box_ballot FOR EACH ROW EXECUTE FUNCTION ballot_box_seal_guard();
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = 'ballot_box_ballot'::regclass
                   AND tgname = 'ballot_box_seal_guard_truncate') THEN
        CREATE TRIGGER ballot_box_seal_guard_truncate BEFORE TRUNCATE ON ballot_box_ballot
            FOR EACH STATEMENT EXECUTE FUNCTION ballot_box_seal_guard_truncate();
    END IF;
END;
$$;
COMMIT;
