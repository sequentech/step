-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

SET LOCAL lock_timeout = '10s';

DROP TRIGGER IF EXISTS "guard_ballot_box_seal_record_policy_value"
ON "sequent_backend"."election_event";
DROP FUNCTION IF EXISTS "sequent_backend"."guard_ballot_box_seal_record_policy_value"();

DROP TRIGGER IF EXISTS "ballot_box_seal_record_document_truncate"
ON "sequent_backend"."document";
DROP FUNCTION IF EXISTS "sequent_backend"."ballot_box_seal_record_document_truncate"();

DROP TRIGGER IF EXISTS "ballot_box_seal_record_document_guard"
ON "sequent_backend"."document";
DROP FUNCTION IF EXISTS "sequent_backend"."ballot_box_seal_record_document_guard"();

-- The permanence rule of 1800.
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
