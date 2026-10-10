-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Ballot box seals: guards of the seal record (VOTE-FREEZE, review R10).
-- - A seal moves to `published` with a public path exactly when the event's
--   effective Seal Record Publication policy is `public`, and without one
--   when it is `restricted` (S2).
-- - The `document` row of a published seal's record can't be changed,
--   deleted or truncated: the record is found through it, and a published
--   seal is never uploaded again (S1).
-- - On a seal-at-close event, `ballot_box_seal_record_policy` is
--   `restricted`, `public`, absent or JSON null on every write (S3). A
--   policy-off event doesn't read it, so its writes are as before.

SET LOCAL lock_timeout = '10s';

-- As in 1800, plus the public path that follows the record policy.
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
    publication_fields text[] := ARRAY[
        'log_entry_id', 'public_document_id', 'public_path', 'published_at'
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
           AND NEW.failure_reason IS NULL AND NEW.log_entry_id IS NULL
           AND NEW.public_document_id IS NULL AND NEW.public_path IS NULL
           AND NEW.published_at IS NULL AND NEW.failure_posted_at IS NULL
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
           AND to_jsonb(NEW) - provenance - seal_fields - 'status'
               = to_jsonb(OLD) - provenance - seal_fields - 'status'
           AND NEW.sealed_at IS NOT NULL AND NEW.ballots_in_box IS NOT NULL
           AND NEW.ballots_counted IS NOT NULL AND NEW.seal_hash IS NOT NULL
           AND NEW.manifest IS NOT NULL AND NEW.signed_message IS NOT NULL THEN
            RETURN NEW;
        END IF;
        IF NEW.status = 'failed'
           AND to_jsonb(NEW) - provenance - 'failure_reason' - 'status'
               - 'election_name' - 'area_name'
               = to_jsonb(OLD) - provenance - 'failure_reason' - 'status'
               - 'election_name' - 'area_name'
           AND NEW.failure_reason IS NOT NULL THEN
            RETURN NEW;
        END IF;
    ELSIF OLD.status = 'sealed' THEN
        IF NEW.status IN ('sealed', 'published')
           AND to_jsonb(NEW) - publication_fields - 'status'
               = to_jsonb(OLD) - publication_fields - 'status'
           AND (OLD.log_entry_id IS NULL OR NEW.log_entry_id IS NOT DISTINCT FROM OLD.log_entry_id)
           AND (OLD.public_document_id IS NULL OR NEW.public_document_id IS NOT DISTINCT FROM OLD.public_document_id)
           AND (OLD.public_path IS NULL OR NEW.public_path IS NOT DISTINCT FROM OLD.public_path)
           AND (OLD.published_at IS NULL OR NEW.published_at IS NOT DISTINCT FROM OLD.published_at)
           AND (NEW.status = 'sealed' OR (
                NEW.log_entry_id IS NOT NULL AND NEW.public_document_id IS NOT NULL
                AND NEW.published_at IS NOT NULL
                -- A public path exactly when the event's record policy is public.
                AND (NEW.public_path IS NOT NULL) = EXISTS (
                    SELECT 1
                    FROM "sequent_backend"."election_event" AS event
                    WHERE event.tenant_id = NEW.tenant_id
                      AND event.id = NEW.election_event_id
                      AND "sequent_backend"."ballot_box_seal_effective_policy"(
                              event.presentation::jsonb, 'ballot_box_seal_record_policy', 'restricted')
                          = '"public"'::jsonb
                )
           )) THEN
            RETURN NEW;
        END IF;
    ELSIF OLD.status = 'failed' THEN
        IF NEW.status = 'failed'
           AND OLD.failure_posted_at IS NULL
           AND NEW.failure_posted_at IS NOT NULL
           AND to_jsonb(NEW) - 'failure_posted_at' = to_jsonb(OLD) - 'failure_posted_at' THEN
            RETURN NEW;
        END IF;
    END IF;

    RAISE EXCEPTION 'The % seal of ballot box % can''t change this way (to %)',
        OLD.status, OLD.id, NEW.status
        USING ERRCODE = '42501',
              HINT = 'A seal only moves pending → sealed → published, or pending → failed; published and failed seals are final.';
END;
$$;

-- A published seal's record document stays as it is.
CREATE OR REPLACE FUNCTION "sequent_backend"."ballot_box_seal_record_document_guard"()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM "sequent_backend"."ballot_box_seal" AS seal
        WHERE seal.public_document_id = OLD.id
          AND seal.status = 'published'
    ) THEN
        RAISE EXCEPTION 'Document % is a published ballot box seal record and can''t be changed or deleted', OLD.id
            USING ERRCODE = '42501',
                  HINT = 'A published seal is final: its record is found through this document.';
    END IF;
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS "ballot_box_seal_record_document_guard"
ON "sequent_backend"."document";

CREATE TRIGGER "ballot_box_seal_record_document_guard"
BEFORE UPDATE OR DELETE
ON "sequent_backend"."document"
FOR EACH ROW
EXECUTE FUNCTION "sequent_backend"."ballot_box_seal_record_document_guard"();

-- TRUNCATE skips the row trigger above.
CREATE OR REPLACE FUNCTION "sequent_backend"."ballot_box_seal_record_document_truncate"()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM "sequent_backend"."ballot_box_seal" AS seal
        WHERE seal.public_document_id IS NOT NULL
          AND seal.status = 'published'
    ) THEN
        RAISE EXCEPTION 'The documents include published ballot box seal records and can''t be truncated'
            USING ERRCODE = '42501',
                  HINT = 'A published seal is final: its record is found through its document.';
    END IF;
    RETURN NULL;
END;
$$;

DROP TRIGGER IF EXISTS "ballot_box_seal_record_document_truncate"
ON "sequent_backend"."document";

CREATE TRIGGER "ballot_box_seal_record_document_truncate"
BEFORE TRUNCATE
ON "sequent_backend"."document"
FOR EACH STATEMENT
EXECUTE FUNCTION "sequent_backend"."ballot_box_seal_record_document_truncate"();

-- The record policy of a seal-at-close event is one the seal can read.
CREATE OR REPLACE FUNCTION "sequent_backend"."guard_ballot_box_seal_record_policy_value"()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    value jsonb := NEW.presentation::jsonb -> 'ballot_box_seal_record_policy';
BEGIN
    IF "sequent_backend"."ballot_box_seal_policy"(NEW.presentation::jsonb) <> 'seal-at-close'
       OR value IS NULL
       OR value = 'null'::jsonb
       OR value IN ('"restricted"'::jsonb, '"public"'::jsonb) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'The ballot_box_seal_record_policy of election event % must be "restricted" or "public", not %',
        NEW.id, value::text
        USING ERRCODE = '22023',
              HINT = 'With the Ballot Box Seal Policy set to Seal at close, the seal record is published by this policy; leave it unset for restricted.';
END;
$$;

DROP TRIGGER IF EXISTS "guard_ballot_box_seal_record_policy_value"
ON "sequent_backend"."election_event";

CREATE TRIGGER "guard_ballot_box_seal_record_policy_value"
BEFORE INSERT OR UPDATE OF presentation
ON "sequent_backend"."election_event"
FOR EACH ROW
EXECUTE FUNCTION "sequent_backend"."guard_ballot_box_seal_record_policy_value"();
