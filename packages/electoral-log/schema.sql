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
-- The signed statement's event type (USER, SYSTEM) and log type (INFO, ERROR),
-- copied out of `message` so that lists can filter by them. They are not part of
-- a record's commitment.
ALTER TABLE electoral_log_messages ADD COLUMN IF NOT EXISTS event_type TEXT;
ALTER TABLE electoral_log_messages ADD COLUMN IF NOT EXISTS log_type TEXT;
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
-- Rejected ballots, which the tally input of an area reads for the voters without a
-- valid ballot (`tally_ballots_query`): few, so it reads them without reading the
-- area's other ballots again.
CREATE INDEX IF NOT EXISTS ballot_box_ballot_rejected
    ON ballot_box_ballot (election_event_id, election_id, area_id, voter_id, seq DESC)
    WHERE status = 'rejected';
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

-- Privacy data deliberately lives outside public: console readers are granted
-- all public tables, including future ones, but must never resolve identities.
CREATE SCHEMA IF NOT EXISTS sequent_backend;
REVOKE ALL ON SCHEMA sequent_backend FROM PUBLIC;
CREATE TABLE IF NOT EXISTS sequent_backend.ballot_identity (
    election_event_id UUID PRIMARY KEY,
    mode TEXT NOT NULL CHECK (mode IN ('voter-id', 'pseudonym')),
    mapping_purged_at TIMESTAMPTZ
);
CREATE TABLE IF NOT EXISTS sequent_backend.ballot_code (
    election_event_id UUID NOT NULL,
    code TEXT NOT NULL CHECK (code ~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'),
    kind TEXT NOT NULL CHECK (kind IN ('pseudonym', 'ballot')),
    PRIMARY KEY (election_event_id, code)
);
CREATE TABLE IF NOT EXISTS sequent_backend.voter_pseudonym (
    election_event_id UUID NOT NULL REFERENCES sequent_backend.ballot_identity,
    voter_id TEXT NOT NULL CHECK (voter_id <> ''),
    pseudonym TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (election_event_id, voter_id),
    UNIQUE (election_event_id, pseudonym),
    FOREIGN KEY (election_event_id, pseudonym)
        REFERENCES sequent_backend.ballot_code (election_event_id, code)
);
CREATE TABLE IF NOT EXISTS sequent_backend.voter_participation (
    election_event_id UUID NOT NULL,
    election_id UUID NOT NULL,
    pseudonym TEXT NOT NULL,
    PRIMARY KEY (election_event_id, election_id, pseudonym),
    FOREIGN KEY (election_event_id, pseudonym)
        REFERENCES sequent_backend.voter_pseudonym (election_event_id, pseudonym) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS voter_participation_pseudonym
    ON sequent_backend.voter_participation (election_event_id, pseudonym, election_id);
CREATE TABLE IF NOT EXISTS sequent_backend.ballot_evidence_pending (
    id BIGSERIAL PRIMARY KEY,
    election_event_id UUID NOT NULL,
    received_id UUID NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('received', 'audited')),
    UNIQUE (election_event_id, received_id, kind)
);
ALTER TABLE sequent_backend.ballot_evidence_pending
    ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ NOT NULL DEFAULT now();
CREATE INDEX IF NOT EXISTS ballot_evidence_pending_event
    ON sequent_backend.ballot_evidence_pending (election_event_id, id);
CREATE TABLE IF NOT EXISTS sequent_backend.privacy_message_pending (
    id BIGSERIAL PRIMARY KEY,
    election_event_id UUID NOT NULL,
    delivery_id TEXT NOT NULL,
    message BYTEA NOT NULL,
    UNIQUE (election_event_id, delivery_id)
);
CREATE INDEX IF NOT EXISTS privacy_message_pending_event
    ON sequent_backend.privacy_message_pending (election_event_id, id);
CREATE TABLE IF NOT EXISTS sequent_backend.forensic_network_record (
    id BIGSERIAL PRIMARY KEY,
    election_event_id UUID NOT NULL,
    election_id UUID,
    area_id UUID,
    event_kind TEXT NOT NULL,
    delivery_id TEXT NOT NULL,
    pseudonym TEXT NOT NULL,
    ballot_id TEXT,
    ip TEXT NOT NULL,
    country TEXT,
    user_agent TEXT,
    recorded_at TIMESTAMPTZ NOT NULL,
    UNIQUE (election_event_id, delivery_id)
);
CREATE TABLE IF NOT EXISTS sequent_backend.forensic_summary_pending (
    record_id BIGINT PRIMARY KEY REFERENCES sequent_backend.forensic_network_record(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS forensic_network_address
    ON sequent_backend.forensic_network_record (election_event_id, ip, recorded_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS forensic_network_retention
    ON sequent_backend.forensic_network_record (election_event_id, recorded_at, id);
CREATE TABLE IF NOT EXISTS sequent_backend.forensic_network_summary (
    election_event_id UUID NOT NULL,
    election_id UUID NOT NULL,
    area_id UUID NOT NULL,
    ip TEXT NOT NULL,
    records BIGINT NOT NULL,
    country TEXT,
    first_at TIMESTAMPTZ NOT NULL,
    last_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (election_event_id, election_id, area_id, ip)
);

CREATE TABLE IF NOT EXISTS sequent_backend.forensic_network_pseudonym (
    election_event_id UUID NOT NULL,
    election_id UUID NOT NULL,
    area_id UUID NOT NULL,
    ip TEXT NOT NULL,
    pseudonym TEXT NOT NULL,
    records BIGINT NOT NULL,
    PRIMARY KEY (election_event_id, election_id, area_id, ip, pseudonym)
);
-- Event-wide address previews must not scan every election/area's pseudonyms.
CREATE INDEX IF NOT EXISTS forensic_pseudonym_address
    ON sequent_backend.forensic_network_pseudonym (election_event_id, ip, pseudonym, election_id);
CREATE INDEX IF NOT EXISTS forensic_summary_address
    ON sequent_backend.forensic_network_summary (election_event_id, ip, election_id);
-- A shared per-session limiter; only an irreversible digest is retained.
CREATE TABLE IF NOT EXISTS sequent_backend.ballot_log_search_limit (
    session_key BYTEA PRIMARY KEY CHECK (octet_length(session_key) = 32),
    searched_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS ballot_log_search_expiry
    ON sequent_backend.ballot_log_search_limit (searched_at);
CREATE INDEX IF NOT EXISTS ballot_box_privacy_page
    ON ballot_box_ballot (election_event_id, area_id, seq DESC) WHERE status = 'valid';
CREATE INDEX IF NOT EXISTS ballot_box_privacy_pseudonym
    ON ballot_box_ballot (election_event_id, area_id, voter_id text_pattern_ops, seq DESC) WHERE status = 'valid';
CREATE INDEX IF NOT EXISTS ballot_box_privacy_received_code
    ON ballot_box_received (election_event_id, area_id, ballot_id text_pattern_ops) WHERE status = 'cast';
CREATE INDEX IF NOT EXISTS ballot_box_received_hash
    ON ballot_box_received (election_event_id, ballot_hash);

CREATE OR REPLACE FUNCTION sequent_backend.immutable_ballot_identity() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' OR NEW.mode <> OLD.mode
       OR (OLD.mapping_purged_at IS NOT NULL AND NEW.mapping_purged_at IS DISTINCT FROM OLD.mapping_purged_at) THEN
        RAISE EXCEPTION 'ballot_identity_immutable' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END;
$$;
CREATE OR REPLACE FUNCTION sequent_backend.guard_private_ballot() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE privacy BOOLEAN;
BEGIN
    SELECT mode = 'pseudonym' INTO privacy FROM sequent_backend.ballot_identity
        WHERE election_event_id = NEW.election_event_id;
    IF NOT COALESCE(privacy, false) THEN RETURN NEW; END IF;
    IF NOT EXISTS (SELECT 1 FROM sequent_backend.voter_pseudonym
                   WHERE election_event_id = NEW.election_event_id AND pseudonym = NEW.voter_id) THEN
        RAISE EXCEPTION 'ballot_pseudonym_not_allocated' USING ERRCODE = '42501';
    END IF;
    IF TG_TABLE_NAME LIKE 'ballot_box_ballot%' AND
       (to_jsonb(NEW)->>'voter_ip' IS NOT NULL OR to_jsonb(NEW)->>'voter_country' IS NOT NULL
        OR (to_jsonb(NEW)->>'username' IS NOT NULL AND to_jsonb(NEW)->>'username' <> NEW.voter_id)) THEN
        RAISE EXCEPTION 'ballot_private_fields_forbidden' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END;
$$;
CREATE OR REPLACE FUNCTION sequent_backend.record_private_participation() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO sequent_backend.voter_participation (election_event_id, election_id, pseudonym)
        SELECT NEW.election_event_id, NEW.election_id, NEW.voter_id
        FROM sequent_backend.ballot_identity
        WHERE election_event_id = NEW.election_event_id AND mode = 'pseudonym'
          AND NEW.status IN ('valid', 'pending')
        ON CONFLICT DO NOTHING;
    RETURN NEW;
END;
$$;
CREATE OR REPLACE FUNCTION sequent_backend.queue_received_evidence() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE entry_kind TEXT;
BEGIN
    IF TG_OP = 'INSERT' THEN entry_kind := 'received';
    ELSIF NEW.status = 'audited' AND OLD.status <> 'audited' THEN entry_kind := 'audited';
    ELSE RETURN NEW;
    END IF;
    INSERT INTO sequent_backend.ballot_evidence_pending (election_event_id, received_id, kind)
        SELECT NEW.election_event_id, NEW.id, entry_kind
        FROM sequent_backend.ballot_identity
        WHERE election_event_id = NEW.election_event_id AND mode = 'pseudonym'
        ON CONFLICT DO NOTHING;
    RETURN NEW;
END;
$$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = 'sequent_backend.ballot_identity'::regclass
                   AND tgname = 'ballot_identity_immutable') THEN
        CREATE TRIGGER ballot_identity_immutable BEFORE UPDATE OR DELETE ON sequent_backend.ballot_identity
            FOR EACH ROW EXECUTE FUNCTION sequent_backend.immutable_ballot_identity();
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = 'ballot_box_ballot'::regclass
                   AND tgname = 'ballot_privacy_guard') THEN
        CREATE TRIGGER ballot_privacy_guard BEFORE INSERT OR UPDATE ON ballot_box_ballot
            FOR EACH ROW EXECUTE FUNCTION sequent_backend.guard_private_ballot();
        CREATE TRIGGER ballot_privacy_participation AFTER INSERT ON ballot_box_ballot
            FOR EACH ROW EXECUTE FUNCTION sequent_backend.record_private_participation();
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = 'ballot_box_received'::regclass
                   AND tgname = 'received_privacy_guard') THEN
        CREATE TRIGGER received_privacy_guard BEFORE INSERT OR UPDATE ON ballot_box_received
            FOR EACH ROW EXECUTE FUNCTION sequent_backend.guard_private_ballot();
        CREATE TRIGGER received_privacy_evidence AFTER INSERT OR UPDATE ON ballot_box_received
            FOR EACH ROW EXECUTE FUNCTION sequent_backend.queue_received_evidence();
    END IF;
END;
$$;


-- Non-identifying evidence survives deletion of the private identities/records.
CREATE TABLE IF NOT EXISTS sequent_backend.privacy_purge_evidence (
    id BIGSERIAL PRIMARY KEY,
    election_event_id UUID NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('mappings', 'network')),
    cutoff TIMESTAMPTZ,
    removed BIGINT NOT NULL CHECK (removed >= 0),
    completed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS sequent_backend.private_receipt_document (
    election_event_id UUID NOT NULL,
    document_id UUID NOT NULL,
    pseudonym TEXT NOT NULL,
    PRIMARY KEY (election_event_id, document_id)
);

COMMIT;
