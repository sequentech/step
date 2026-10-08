-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

SET LOCAL lock_timeout = '10s';

DROP TRIGGER IF EXISTS "guard_ballot_box_seal_closed_channels"
ON "sequent_backend"."election";
DROP FUNCTION IF EXISTS "sequent_backend"."guard_ballot_box_seal_closed_channels"();

DROP TRIGGER IF EXISTS "guard_ballot_box_seal_event_settings"
ON "sequent_backend"."election_event";
DROP FUNCTION IF EXISTS "sequent_backend"."guard_ballot_box_seal_event_settings"();
DROP FUNCTION IF EXISTS "sequent_backend"."ballot_box_seal_voting_opened"(uuid, uuid, jsonb);
DROP FUNCTION IF EXISTS "sequent_backend"."ballot_box_seal_effective_policy"(jsonb, text, text);

-- The guard and the permanence rule of 1600.
CREATE OR REPLACE FUNCTION "sequent_backend"."cast_vote_seal_guard"()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    old_tenant uuid;
    old_event uuid;
    old_election uuid;
    old_area uuid;
    new_tenant uuid;
    new_event uuid;
    new_election uuid;
    new_area uuid;
    box record;
BEGIN
    IF TG_OP IN ('UPDATE', 'DELETE') THEN
        old_tenant := OLD.tenant_id;
        old_event := OLD.election_event_id;
        old_election := OLD.election_id;
        old_area := OLD.area_id;
    END IF;
    IF TG_OP IN ('INSERT', 'UPDATE') THEN
        new_tenant := NEW.tenant_id;
        new_event := NEW.election_event_id;
        new_election := NEW.election_id;
        new_area := NEW.area_id;
    END IF;

    FOR box IN
        SELECT DISTINCT
            candidate.tenant_id,
            candidate.election_event_id,
            candidate.election_id,
            candidate.area_id,
            "sequent_backend"."ballot_box_lock_key"(
                candidate.tenant_id, candidate.election_event_id,
                candidate.election_id, candidate.area_id
            ) AS lock_key
        FROM (VALUES
            (old_tenant, old_event, old_election, old_area),
            (new_tenant, new_event, new_election, new_area)
        ) AS candidate(tenant_id, election_event_id, election_id, area_id)
        WHERE candidate.tenant_id IS NOT NULL
          AND candidate.election_event_id IS NOT NULL
          AND candidate.election_id IS NOT NULL
          AND candidate.area_id IS NOT NULL
        ORDER BY lock_key
    LOOP
        -- Above READ COMMITTED the check below would read the transaction's
        -- snapshot, taken before the lock wait, and miss a seal committed
        -- during it. So boxes of a seal-at-close event only take writes at
        -- READ COMMITTED. The policy can't change once voting opened, so the
        -- snapshot's value is reliable.
        IF current_setting('transaction_isolation') <> 'read committed'
           AND EXISTS (
               SELECT 1
               FROM "sequent_backend"."election_event" AS event
               WHERE event.id = box.election_event_id
                 AND event.tenant_id = box.tenant_id
                 AND "sequent_backend"."ballot_box_seal_policy"(event.presentation::jsonb)
                     = 'seal-at-close'
           ) THEN
            RAISE EXCEPTION 'ballot_box_seal_requires_read_committed'
                USING ERRCODE = '42501',
                      DETAIL = format('Writes to cast_vote of a seal-at-close event must run at READ COMMITTED, not %s.',
                                      current_setting('transaction_isolation')),
                      HINT = 'Run the transaction at READ COMMITTED isolation.';
        END IF;
        PERFORM pg_advisory_xact_lock_shared(hashtextextended(box.lock_key, 0));
        IF EXISTS (
            SELECT 1
            FROM "sequent_backend"."ballot_box_seal" AS seal
            WHERE seal.tenant_id = box.tenant_id
              AND seal.election_event_id = box.election_event_id
              AND seal.election_id = box.election_id
              AND seal.area_id = box.area_id
              AND seal.status <> 'pending'
        ) THEN
            RAISE EXCEPTION 'ballot_box_sealed'
                USING ERRCODE = '42501',
                      DETAIL = format('The ballot box of election %s, area %s is sealed.',
                                      box.election_id, box.area_id),
                      HINT = 'A sealed ballot box takes no new ballots and no changes.';
        END IF;
    END LOOP;

    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END;
$$;

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
           AND to_jsonb(NEW) - provenance - seal_fields - 'status'
               = to_jsonb(OLD) - provenance - seal_fields - 'status'
           AND NEW.sealed_at IS NOT NULL AND NEW.ballots_in_box IS NOT NULL
           AND NEW.ballots_counted IS NOT NULL AND NEW.seal_hash IS NOT NULL
           AND NEW.manifest IS NOT NULL AND NEW.signed_message IS NOT NULL THEN
            RETURN NEW;
        END IF;
        IF NEW.status = 'failed'
           AND to_jsonb(NEW) - provenance - 'failure_reason' - 'status'
               = to_jsonb(OLD) - provenance - 'failure_reason' - 'status'
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

DROP INDEX IF EXISTS "sequent_backend"."ballot_box_seal_failure_unposted_idx";

ALTER TABLE "sequent_backend"."ballot_box_seal"
    DROP COLUMN IF EXISTS "area_name",
    DROP COLUMN IF EXISTS "election_name",
    DROP COLUMN IF EXISTS "failure_posted_at",
    DROP COLUMN IF EXISTS "waiting_reason",
    DROP COLUMN IF EXISTS "last_attempt_at";
