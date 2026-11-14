-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Deleting an election event moves it to Trash. Its data stays until
-- purge_after, when the beat's scheduled_events task claims it
-- (purge_started_at) and deletes it permanently. A claimed event can no
-- longer be restored. An event in Trash is archived, so every archive guard
-- applies to it, and archived_before_trash is what restoring puts back.
ALTER TABLE "sequent_backend"."election_event"
    ADD COLUMN "trashed_at" timestamptz,
    ADD COLUMN "purge_after" timestamptz,
    ADD COLUMN "archived_before_trash" boolean,
    ADD COLUMN "purge_started_at" timestamptz,
    ADD CONSTRAINT "election_event_trash_state" CHECK (
        ("trashed_at" IS NULL) = ("purge_after" IS NULL)
        AND ("trashed_at" IS NULL) = ("archived_before_trash" IS NULL)
        AND ("purge_started_at" IS NULL OR "trashed_at" IS NOT NULL)
        AND ("trashed_at" IS NULL OR "is_archived" IS TRUE)
    );

CREATE INDEX "election_event_purge_after_idx"
    ON "sequent_backend"."election_event" ("purge_after")
    WHERE "trashed_at" IS NOT NULL;

-- Voting status can't change, and elections can't be added, while the
-- election event is in Trash, whatever connection writes them. Moving an
-- event to Trash and restoring it hold its row FOR UPDATE; an election's
-- writer takes FOR KEY SHARE on its event here, so it waits for them and then
-- reads the event as they left it, and they wait for the writers that hold it
-- and then read what those committed. FOR KEY SHARE doesn't conflict with the
-- election event's own updates, so status writers never wait on each other
-- because of it.
CREATE OR REPLACE FUNCTION "sequent_backend"."guard_trashed_election_event_status"()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    event_id uuid;
    in_trash boolean;
BEGIN
    IF TG_TABLE_NAME = 'election_event' THEN
        event_id := NEW.id;
        in_trash := NEW.trashed_at IS NOT NULL;
    ELSE
        event_id := NEW.election_event_id;
        SELECT "trashed_at" IS NOT NULL
        INTO in_trash
        FROM "sequent_backend"."election_event"
        WHERE "id" = event_id
          AND "tenant_id" = NEW.tenant_id
        FOR KEY SHARE;
    END IF;

    IF in_trash THEN
        RAISE EXCEPTION 'Election event % is in Trash; restore it before changing it',
            event_id
            USING ERRCODE = '55000';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER "guard_trashed_election_event_status"
BEFORE UPDATE OF "status" ON "sequent_backend"."election_event"
FOR EACH ROW
WHEN (OLD."status" IS DISTINCT FROM NEW."status")
EXECUTE FUNCTION "sequent_backend"."guard_trashed_election_event_status"();

CREATE TRIGGER "guard_trashed_election_event_status"
BEFORE UPDATE OF "status" ON "sequent_backend"."election"
FOR EACH ROW
WHEN (OLD."status" IS DISTINCT FROM NEW."status")
EXECUTE FUNCTION "sequent_backend"."guard_trashed_election_event_status"();

CREATE TRIGGER "guard_trashed_election_event_insert"
BEFORE INSERT ON "sequent_backend"."election"
FOR EACH ROW
EXECUTE FUNCTION "sequent_backend"."guard_trashed_election_event_status"();
