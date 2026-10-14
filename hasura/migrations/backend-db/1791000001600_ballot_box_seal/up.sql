-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Ballot box seals (VOTE-FREEZE). With the event's `ballot_box_seal_policy`
-- at `seal-at-close`, closing an election creates one `pending` seal per
-- ballot box (the ballots of one election and area in the event's ballot
-- box). Once its grace period ends the sealer hashes and signs the box and
-- stores the seal, with its record on the event's electoral log, in the
-- event's electoral-log database, whose guard refuses every later write to
-- the box. This table follows that seal: it is the sealer's work list and
-- what the admin portal reads. A seal moves `pending → sealed → published`
-- or `pending → failed`; a failed seal keeps the box locked too.

CREATE TABLE "sequent_backend"."ballot_box_seal" (
    "id" uuid NOT NULL DEFAULT gen_random_uuid(),
    "tenant_id" uuid NOT NULL,
    "election_event_id" uuid NOT NULL,
    "election_id" uuid NOT NULL,
    "area_id" uuid NOT NULL,
    "status" text NOT NULL DEFAULT 'pending',
    "closed_at" timestamptz NOT NULL,
    "grace_deadline" timestamptz NOT NULL,
    "close_request_id" uuid NULL,
    "closed_by" jsonb NOT NULL,
    "sealed_at" timestamptz NULL,
    "ballots_in_box" bigint NULL,
    "ballots_counted" bigint NULL,
    "seal_hash" text NULL,
    "manifest" bytea NULL,
    "signed_message" bytea NULL,
    "failure_reason" text NULL,
    -- The seal's record on the event's electoral log: its delivery ID, the
    -- log it is in and its leaf there, which name it wherever the log is
    -- copied.
    "log_delivery_id" text NULL,
    "log_uid" uuid NULL,
    "log_leaf_index" bigint NULL,
    "public_document_id" uuid NULL,
    "public_path" text NULL,
    "published_at" timestamptz NULL,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY ("id"),
    -- Exactly one seal per ballot box.
    CONSTRAINT "ballot_box_seal_box_key"
        UNIQUE ("tenant_id", "election_event_id", "election_id", "area_id"),
    CONSTRAINT "ballot_box_seal_status_check"
        CHECK ("status" IN ('pending', 'sealed', 'published', 'failed'))
);

-- The sealer's and publisher's work lists.
CREATE INDEX "ballot_box_seal_open_work_idx"
    ON "sequent_backend"."ballot_box_seal" ("status", "grace_deadline")
    WHERE "status" IN ('pending', 'sealed');

-- A seal is a permanent record. Inserts start `pending` with no seal,
-- failure or publication data; deletes are refused; identity and close
-- times never change. While `pending` only `close_request_id` and
-- `closed_by` change (the signed close completes them), plus the move to
-- `sealed` (with all its seal fields and its record on the electoral log)
-- or to `failed` (with its reason, and its record when it has one).
-- While `sealed` the publication fields are set once each, and the move to
-- `published` needs all of them. `published` and `failed` are terminal.
CREATE OR REPLACE FUNCTION "sequent_backend"."ballot_box_seal_permanence"()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    -- Columns a pending seal may change while it stays pending.
    provenance text[] := ARRAY['close_request_id', 'closed_by'];
    seal_fields text[] := ARRAY[
        'sealed_at', 'ballots_in_box', 'ballots_counted', 'seal_hash',
        'manifest', 'signed_message'
    ];
    -- The seal's (or the failure's) record on the event's electoral log.
    log_fields text[] := ARRAY['log_delivery_id', 'log_uid', 'log_leaf_index'];
    publication_fields text[] := ARRAY[
        'public_document_id', 'public_path', 'published_at'
    ];
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'The seal of ballot box % is permanent and can''t be deleted', OLD.id
            USING ERRCODE = '42501',
                  HINT = 'Ballot box seals are kept for the audit of the election.';
    END IF;

    IF TG_OP = 'INSERT' THEN
        IF NEW.status = 'pending'
           AND NEW.sealed_at IS NULL AND NEW.ballots_in_box IS NULL
           AND NEW.ballots_counted IS NULL AND NEW.seal_hash IS NULL
           AND NEW.manifest IS NULL AND NEW.signed_message IS NULL
           AND NEW.failure_reason IS NULL AND NEW.log_delivery_id IS NULL
           AND NEW.log_uid IS NULL AND NEW.log_leaf_index IS NULL
           AND NEW.public_document_id IS NULL AND NEW.public_path IS NULL
           AND NEW.published_at IS NULL THEN
            RETURN NEW;
        END IF;
        RAISE EXCEPTION 'A new ballot box seal can only start pending, without seal data'
            USING ERRCODE = '42501',
                  HINT = 'Seals are created when voting closes and sealed by the server.';
    END IF;

    IF OLD.status = 'pending' THEN
        IF NEW.status = 'pending'
           AND to_jsonb(NEW) - provenance = to_jsonb(OLD) - provenance THEN
            RETURN NEW;
        END IF;
        IF NEW.status = 'sealed'
           AND to_jsonb(NEW) - provenance - seal_fields - log_fields - 'status'
               = to_jsonb(OLD) - provenance - seal_fields - log_fields - 'status'
           AND NEW.sealed_at IS NOT NULL AND NEW.ballots_in_box IS NOT NULL
           AND NEW.ballots_counted IS NOT NULL AND NEW.seal_hash IS NOT NULL
           AND NEW.manifest IS NOT NULL AND NEW.signed_message IS NOT NULL
           AND NEW.log_delivery_id IS NOT NULL AND NEW.log_uid IS NOT NULL
           AND NEW.log_leaf_index IS NOT NULL THEN
            RETURN NEW;
        END IF;
        IF NEW.status = 'failed'
           AND to_jsonb(NEW) - provenance - log_fields - 'failure_reason' - 'status'
               = to_jsonb(OLD) - provenance - log_fields - 'failure_reason' - 'status'
           AND NEW.failure_reason IS NOT NULL
           AND (NEW.log_delivery_id IS NULL) = (NEW.log_uid IS NULL)
           AND (NEW.log_uid IS NULL) = (NEW.log_leaf_index IS NULL) THEN
            RETURN NEW;
        END IF;
    ELSIF OLD.status = 'sealed' THEN
        IF NEW.status IN ('sealed', 'published')
           AND to_jsonb(NEW) - publication_fields - 'status'
               = to_jsonb(OLD) - publication_fields - 'status'
           AND (OLD.public_document_id IS NULL OR NEW.public_document_id IS NOT DISTINCT FROM OLD.public_document_id)
           AND (OLD.public_path IS NULL OR NEW.public_path IS NOT DISTINCT FROM OLD.public_path)
           AND (OLD.published_at IS NULL OR NEW.published_at IS NOT DISTINCT FROM OLD.published_at)
           AND (NEW.status = 'sealed' OR (
                NEW.public_document_id IS NOT NULL
                AND NEW.public_path IS NOT NULL AND NEW.published_at IS NOT NULL
           )) THEN
            RETURN NEW;
        END IF;
    END IF;

    RAISE EXCEPTION 'The % seal of ballot box % can''t change this way (to %)',
        OLD.status, OLD.id, NEW.status
        USING ERRCODE = '42501',
              HINT = 'A seal only moves pending → sealed → published, or pending → failed; published and failed seals are final.';
END;
$$;

CREATE OR REPLACE FUNCTION "sequent_backend"."ballot_box_seal_refuse_truncate"()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'Ballot box seals are permanent and can''t be truncated'
        USING ERRCODE = '42501';
END;
$$;

DROP TRIGGER IF EXISTS "ballot_box_seal_permanence"
ON "sequent_backend"."ballot_box_seal";

CREATE TRIGGER "ballot_box_seal_permanence"
BEFORE INSERT OR UPDATE OR DELETE
ON "sequent_backend"."ballot_box_seal"
FOR EACH ROW
EXECUTE FUNCTION "sequent_backend"."ballot_box_seal_permanence"();

-- TRUNCATE skips the row trigger above.
DROP TRIGGER IF EXISTS "ballot_box_seal_permanence_truncate"
ON "sequent_backend"."ballot_box_seal";

CREATE TRIGGER "ballot_box_seal_permanence_truncate"
BEFORE TRUNCATE
ON "sequent_backend"."ballot_box_seal"
FOR EACH STATEMENT
EXECUTE FUNCTION "sequent_backend"."ballot_box_seal_refuse_truncate"();

-- The seal policy as ElectionEventPresentation reads it: only
-- `seal-at-close` seals, anything else (missing, null, unknown) doesn't.
CREATE OR REPLACE FUNCTION "sequent_backend"."ballot_box_seal_policy"(presentation jsonb)
RETURNS text
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT CASE
        WHEN presentation ->> 'ballot_box_seal_policy' = 'seal-at-close' THEN 'seal-at-close'
        ELSE 'do-not-seal'
    END;
$$;

-- The seal policy is locked once voting has opened: on the event (any
-- channel status other than NOT_STARTED, or any period date) or on any of
-- its elections (a `first_started_at`, or any other period date or status,
-- on any channel). There is no trusted-write exception: not even the server
-- changes it after voting opened.
CREATE OR REPLACE FUNCTION "sequent_backend"."guard_ballot_box_seal_policy_update"()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF "sequent_backend"."ballot_box_seal_policy"(OLD.presentation::jsonb)
        IS NOT DISTINCT FROM
       "sequent_backend"."ballot_box_seal_policy"(NEW.presentation::jsonb) THEN
        RETURN NEW;
    END IF;

    IF NOT "sequent_backend"."trusted_voting_started"(OLD.status::jsonb)
       AND NOT EXISTS (
           SELECT 1
           FROM "sequent_backend"."election" AS election
           WHERE election.tenant_id = OLD.tenant_id
             AND election.election_event_id = OLD.id
             AND "sequent_backend"."trusted_voting_started"(election.status::jsonb)
       ) THEN
        RETURN NEW;
    END IF;

    RAISE EXCEPTION 'The ballot box seal policy of election event % can only change before voting opens',
        OLD.id
        USING ERRCODE = '42501',
              HINT = 'Choose the ballot box seal policy before opening voting.';
END;
$$;

DROP TRIGGER IF EXISTS "guard_ballot_box_seal_policy_update"
ON "sequent_backend"."election_event";

CREATE TRIGGER "guard_ballot_box_seal_policy_update"
BEFORE UPDATE OF presentation
ON "sequent_backend"."election_event"
FOR EACH ROW
EXECUTE FUNCTION "sequent_backend"."guard_ballot_box_seal_policy_update"();

-- Seals are permanent: an election with any (in any status) can't be
-- deleted, also through Hasura (VOTE-FREEZE). Its event is archived instead.
CREATE OR REPLACE FUNCTION "sequent_backend"."election_seal_guard"()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM "sequent_backend"."ballot_box_seal"
        WHERE tenant_id = OLD.tenant_id
          AND election_event_id = OLD.election_event_id
          AND election_id = OLD.id
    ) THEN
        RAISE EXCEPTION 'ballot_box_sealed'
            USING ERRCODE = '42501',
                  DETAIL = 'This election has sealed ballot boxes and cannot be deleted.',
                  HINT = 'Archive the election event instead.';
    END IF;
    RETURN OLD;
END;
$$;

DROP TRIGGER IF EXISTS "election_seal_guard"
ON "sequent_backend"."election";

CREATE TRIGGER "election_seal_guard"
BEFORE DELETE
ON "sequent_backend"."election"
FOR EACH ROW
EXECUTE FUNCTION "sequent_backend"."election_seal_guard"();
