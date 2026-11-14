-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Refused while an election event's permanent deletion has started: its
-- external data may be gone already, and without the claim it would become
-- an ordinary archived event that could be unarchived. Stop the Trash purges
-- and let the claimed ones finish first. The lock keeps new claims out until
-- this migration's transaction ends. Election events still in Trash and not
-- claimed stay archived.
DO $$
BEGIN
    LOCK TABLE "sequent_backend"."election_event" IN SHARE ROW EXCLUSIVE MODE;
    IF EXISTS (
        SELECT 1 FROM "sequent_backend"."election_event"
        WHERE "purge_started_at" IS NOT NULL
    ) THEN
        RAISE EXCEPTION 'Election events are being deleted permanently from Trash; let their purge finish before rolling back'
            USING ERRCODE = '55000';
    END IF;
END;
$$;

DROP TRIGGER IF EXISTS "guard_trashed_election_event_insert"
ON "sequent_backend"."election";

DROP TRIGGER IF EXISTS "guard_trashed_election_event_status"
ON "sequent_backend"."election";

DROP TRIGGER IF EXISTS "guard_trashed_election_event_status"
ON "sequent_backend"."election_event";

DROP FUNCTION IF EXISTS "sequent_backend"."guard_trashed_election_event_status"();

DROP INDEX IF EXISTS "sequent_backend"."election_event_purge_after_idx";

ALTER TABLE "sequent_backend"."election_event"
    DROP CONSTRAINT IF EXISTS "election_event_trash_state",
    DROP COLUMN IF EXISTS "purge_started_at",
    DROP COLUMN IF EXISTS "archived_before_trash",
    DROP COLUMN IF EXISTS "purge_after",
    DROP COLUMN IF EXISTS "trashed_at";
