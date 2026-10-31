-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Ballot box seals, hardening (VOTE-FREEZE review R6).
-- - The sealer records its last attempt on a pending seal (when, and why
--   the box isn't sealed yet), and the election and area names it signed.
-- - For seal-at-close events, once voting has opened: the contest
--   encryption, delegated and weighted voting policies and the bulletin board
--   reference can't change, and a CLOSED voting channel stays CLOSED.

-- Trigger DDL on busy tables must not queue behind long transactions.
SET LOCAL lock_timeout = '10s';

ALTER TABLE "sequent_backend"."ballot_box_seal"
    ADD COLUMN IF NOT EXISTS "last_attempt_at" timestamptz NULL,
    ADD COLUMN IF NOT EXISTS "waiting_reason" text NULL,
    -- The election and area names the seal signed, for its public record.
    ADD COLUMN IF NOT EXISTS "election_name" text NULL,
    ADD COLUMN IF NOT EXISTS "area_name" text NULL;

-- As in 1600, plus: while `pending`, the sealer's last attempt
-- (`last_attempt_at`, `waiting_reason`) may change too, also on the move to
-- `sealed` or `failed`; the names are set on the move to `sealed` (the
-- names it signed) or `failed` (for the incident banner) only.
CREATE OR REPLACE FUNCTION "sequent_backend"."ballot_box_seal_permanence"()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    -- Columns a pending seal may change while it stays pending.
    provenance text[] := ARRAY['close_request_id', 'closed_by', 'last_attempt_at', 'waiting_reason'];
    seal_fields text[] := ARRAY[
        'sealed_at', 'ballots_in_box', 'ballots_counted', 'seal_hash',
        'manifest', 'signed_message', 'election_name', 'area_name'
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
           AND NEW.published_at IS NULL
           AND NEW.election_name IS NULL AND NEW.area_name IS NULL THEN
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
               - 'election_name' - 'area_name'
               = to_jsonb(OLD) - provenance - log_fields - 'failure_reason' - 'status'
               - 'election_name' - 'area_name'
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

-- Whether the event's voting has opened: on the event, or on any of its
-- elections (as the 1600 policy lock reads it).
CREATE OR REPLACE FUNCTION "sequent_backend"."ballot_box_seal_voting_opened"(
    tenant_id uuid, election_event_id uuid, event_status jsonb
)
RETURNS boolean
LANGUAGE sql
STABLE
AS $$
    SELECT "sequent_backend"."trusted_voting_started"(event_status)
        OR EXISTS (
            SELECT 1
            FROM "sequent_backend"."election" AS election
            WHERE election.tenant_id = ballot_box_seal_voting_opened.tenant_id
              AND election.election_event_id = ballot_box_seal_voting_opened.election_event_id
              AND "sequent_backend"."trusted_voting_started"(election.status::jsonb)
        );
$$;

-- A presentation policy's effective value, as the Rust accessors read it:
-- a missing key or JSON null is the default.
CREATE OR REPLACE FUNCTION "sequent_backend"."ballot_box_seal_effective_policy"(
    presentation jsonb, policy_key text, default_value text
)
RETURNS jsonb
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT COALESCE(NULLIF(presentation -> policy_key, 'null'::jsonb), to_jsonb(default_value));
$$;

-- For a seal-at-close event, once voting has opened, what the seal and the
-- tally rely on stays as it was: the policies that decide how a ballot is
-- read and how much it counts (their effective values, with the defaults of
-- ContestEncryptionPolicy, DelegatedVotingPolicy and WeightedVotingPolicy),
-- and the bulletin board the seals go to.
CREATE OR REPLACE FUNCTION "sequent_backend"."guard_ballot_box_seal_event_settings"()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    locked_keys text[] := ARRAY[
        'contest_encryption_policy', 'delegated_voting_policy', 'weighted_voting_policy'
    ];
    locked_defaults text[] := ARRAY[
        'single-contest', 'disabled', 'disabled-weighted-voting'
    ];
    position integer;
BEGIN
    IF "sequent_backend"."ballot_box_seal_policy"(OLD.presentation::jsonb) <> 'seal-at-close'
       OR NOT "sequent_backend"."ballot_box_seal_voting_opened"(OLD.tenant_id, OLD.id, OLD.status::jsonb) THEN
        RETURN NEW;
    END IF;
    FOR position IN 1 .. array_length(locked_keys, 1) LOOP
        IF "sequent_backend"."ballot_box_seal_effective_policy"(
               OLD.presentation::jsonb, locked_keys[position], locked_defaults[position])
           IS DISTINCT FROM
           "sequent_backend"."ballot_box_seal_effective_policy"(
               NEW.presentation::jsonb, locked_keys[position], locked_defaults[position]) THEN
            RAISE EXCEPTION 'The % of election event % can''t change after voting has opened',
                locked_keys[position], OLD.id
                USING ERRCODE = '42501',
                      HINT = 'With the Ballot Box Seal Policy set to Seal at close, the seal relies on it.';
        END IF;
    END LOOP;
    IF OLD.bulletin_board_reference::jsonb IS DISTINCT FROM NEW.bulletin_board_reference::jsonb THEN
        RAISE EXCEPTION 'The bulletin board of election event % can''t change after voting has opened',
            OLD.id
            USING ERRCODE = '42501',
                  HINT = 'With the Ballot Box Seal Policy set to Seal at close, the seals are posted to it.';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS "guard_ballot_box_seal_event_settings"
ON "sequent_backend"."election_event";

CREATE TRIGGER "guard_ballot_box_seal_event_settings"
BEFORE UPDATE OF presentation, bulletin_board_reference
ON "sequent_backend"."election_event"
FOR EACH ROW
EXECUTE FUNCTION "sequent_backend"."guard_ballot_box_seal_event_settings"();

-- For a seal-at-close event, a CLOSED voting channel of an election stays
-- CLOSED: a stale read-modify-write can't reopen it.
CREATE OR REPLACE FUNCTION "sequent_backend"."guard_ballot_box_seal_closed_channels"()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    channel_key text;
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM "sequent_backend"."election_event" AS event
        WHERE event.id = OLD.election_event_id
          AND event.tenant_id = OLD.tenant_id
          AND "sequent_backend"."ballot_box_seal_policy"(event.presentation::jsonb) = 'seal-at-close'
    ) THEN
        RETURN NEW;
    END IF;
    FOREACH channel_key IN ARRAY ARRAY[
        'voting_status', 'kiosk_voting_status', 'early_voting_status', 'telephone_voting_status'
    ] LOOP
        IF OLD.status::jsonb ->> channel_key = 'CLOSED'
           AND (NEW.status::jsonb ->> channel_key) IS DISTINCT FROM 'CLOSED' THEN
            RAISE EXCEPTION 'ballot_box_seal_closed_is_final'
                USING ERRCODE = '42501',
                      DETAIL = format('The %s of election %s is CLOSED and can''t change.',
                                      channel_key, OLD.id),
                      HINT = 'With the Ballot Box Seal Policy set to Seal at close, closed voting stays closed.';
        END IF;
    END LOOP;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS "guard_ballot_box_seal_closed_channels"
ON "sequent_backend"."election";

CREATE TRIGGER "guard_ballot_box_seal_closed_channels"
BEFORE UPDATE OF status
ON "sequent_backend"."election"
FOR EACH ROW
EXECUTE FUNCTION "sequent_backend"."guard_ballot_box_seal_closed_channels"();
