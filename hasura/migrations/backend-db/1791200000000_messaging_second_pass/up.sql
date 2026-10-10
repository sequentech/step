-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Messaging second pass (meta#13755).

-- The event's messaging configuration. Only the update_event_messaging_config
-- action (and an event import, with the same validation and permission)
-- writes it, so the configuration can't be changed around its validation.
-- Replaces the `messaging:config` annotation, which is copied below once and
-- then left untouched. Copied rows are marked `updated_by = 'migration'`;
-- windmill validates them once and marks those a save would refuse
-- ('migration: needs saving again').
CREATE TABLE "sequent_backend"."messaging_event_config"
(
    "tenant_id"         uuid        NOT NULL,
    "election_event_id" uuid        NOT NULL,
    "config"            jsonb       NOT NULL,
    "version"           integer     NOT NULL DEFAULT 1,
    "updated_at"        timestamptz NOT NULL DEFAULT now(),
    "updated_by"        text,
    PRIMARY KEY ("election_event_id"),
    UNIQUE ("tenant_id", "election_event_id"),
    FOREIGN KEY ("tenant_id")
        REFERENCES "sequent_backend"."tenant" ("id")
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    FOREIGN KEY ("tenant_id", "election_event_id")
        REFERENCES "sequent_backend"."election_event" ("tenant_id", "id")
        ON UPDATE RESTRICT
        ON DELETE CASCADE
);

-- The annotation holds the configuration as a JSON string. One that doesn't
-- parse is reported and skipped: the event then has no configuration, as
-- the code already treated it.
DO $$
DECLARE
    event record;
BEGIN
    FOR event IN
        SELECT "tenant_id", "id", "annotations" ->> 'messaging:config' AS "raw"
        FROM "sequent_backend"."election_event"
        WHERE "annotations" ? 'messaging:config'
          AND "tenant_id" IS NOT NULL
    LOOP
        BEGIN
            INSERT INTO "sequent_backend"."messaging_event_config"
                ("tenant_id", "election_event_id", "config", "updated_by")
            VALUES (event."tenant_id", event."id", event."raw"::jsonb, 'migration')
            ON CONFLICT ("election_event_id") DO NOTHING;
        EXCEPTION WHEN invalid_text_representation THEN
            RAISE NOTICE 'messaging:config of election event % is not JSON; not copied', event."id";
        END;
    END LOOP;
END $$;

-- Shared per-account sending capacity, for bulk workers and OTP sends in
-- every process. Updated with one UPDATE ... RETURNING per take.
CREATE TABLE "sequent_backend"."messaging_rate_bucket"
(
    "account_id"  uuid             NOT NULL,
    "tokens"      double precision NOT NULL,
    "refilled_at" timestamptz      NOT NULL DEFAULT now(),
    PRIMARY KEY ("account_id"),
    FOREIGN KEY ("account_id")
        REFERENCES "sequent_backend"."messaging_account" ("id")
        ON UPDATE CASCADE
        ON DELETE CASCADE
);

-- When a Messenger recipient last pressed Get Started or opened a referral
-- for an account. Opens the 24 hour window; voter text no longer does, and
-- inbound messages are no longer stored. Keyed by the recipient's digest,
-- like the ledger.
CREATE TABLE "sequent_backend"."messenger_contact"
(
    "tenant_id"           uuid        NOT NULL,
    "account_id"          uuid        NOT NULL,
    "destination_digest"  text        NOT NULL,
    "last_get_started_at" timestamptz NOT NULL,
    "updated_at"          timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY ("account_id", "destination_digest"),
    FOREIGN KEY ("tenant_id")
        REFERENCES "sequent_backend"."tenant" ("id")
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    FOREIGN KEY ("account_id")
        REFERENCES "sequent_backend"."messaging_account" ("id")
        ON UPDATE CASCADE
        ON DELETE CASCADE
);

-- Ledger: same-channel retries, never-sent vs maybe-sent, reconciliation
-- backoff, billing, and the electoral log's outbox (`logged_at`: when the
-- attempt's entry was handed to the log; NULL until then).
ALTER TABLE "sequent_backend"."message"
    ADD COLUMN "next_retry_at"      timestamptz,
    ADD COLUMN "dispatched_at"      timestamptz,
    ADD COLUMN "last_reconciled_at" timestamptz,
    ADD COLUMN "billing_category"   text,
    ADD COLUMN "billable"           boolean,
    -- Existing rows count as logged (the first release logged them, or
    -- never will); new rows start unlogged.
    ADD COLUMN "logged_at"          timestamptz DEFAULT now();
ALTER TABLE "sequent_backend"."message"
    ALTER COLUMN "logged_at" DROP DEFAULT;

-- A voter with no contact channel gets one FAILED row with no channel
-- ("No contact channel"), never counted as any channel's failure.
ALTER TABLE "sequent_backend"."message"
    ALTER COLUMN "channel" DROP NOT NULL;

-- The first release recorded QUEUED before calling the provider, so an
-- attempt still QUEUED here may have been sent when a deploy stopped its
-- worker. It is maybe-sent: the sweep makes it UNKNOWN, never resends it.
UPDATE "sequent_backend"."message"
    SET "dispatched_at" = "created_at"
    WHERE "state" = 'QUEUED' AND "dispatched_at" IS NULL;

CREATE INDEX "message_unlogged"
    ON "sequent_backend"."message" ("created_at", "id")
    WHERE "logged_at" IS NULL;

-- The latest attempt of each message: what a message's outcome is (the
-- voter drawer's last failure, the Logs filter, the event tab's counts).
-- An attempt is superseded by a later one of the same logical message.
CREATE VIEW "sequent_backend"."message_latest_attempt" AS
SELECT message.*
FROM "sequent_backend"."message" AS message
WHERE message."logical_key" IS NULL
   OR NOT EXISTS (
        SELECT 1 FROM "sequent_backend"."message" AS later
        WHERE later."tenant_id" = message."tenant_id"
          AND later."logical_key" = message."logical_key"
          AND later."attempt" > message."attempt"
   );

CREATE INDEX "message_retry_due"
    ON "sequent_backend"."message" ("next_retry_at")
    WHERE "next_retry_at" IS NOT NULL;
CREATE INDEX "message_unresolved_keyset"
    ON "sequent_backend"."message" ("created_at", "id")
    WHERE "state" IN ('QUEUED', 'UNKNOWN');

-- Voters' messages are no longer kept: delete the ones already stored.
DELETE FROM "sequent_backend"."message" WHERE "direction" = 'INBOUND';

-- The Messenger linking word is gone; the column stays, unused.
ALTER TABLE "sequent_backend"."messenger_link"
    ALTER COLUMN "link_word_digest" DROP NOT NULL;

-- Voters by channel, per event: counted by windmill's recount task (on
-- demand, after a configuration save, and from beat) and read by the event
-- tab and Send Notification, so no request counts the realm. `scope`: the
-- send audience (ALL, VOTED, NOT_VOTED); `area_id` NULL: voters with no
-- area; `channel` NULL: voters with no contact channel. A primary key can't
-- hold NULLs, so the key is a NULLS NOT DISTINCT unique constraint.
CREATE TABLE "sequent_backend"."messaging_voter_count"
(
    "tenant_id"         uuid        NOT NULL,
    "election_event_id" uuid        NOT NULL,
    "scope"             text        NOT NULL,
    "area_id"           text,
    "channel"           text,
    "voters"            bigint      NOT NULL,
    "not_offered"       bigint      NOT NULL,
    "computed_at"       timestamptz NOT NULL,
    CONSTRAINT "messaging_voter_count_key"
        UNIQUE NULLS NOT DISTINCT ("election_event_id", "scope", "area_id", "channel"),
    CHECK ("scope" IN ('ALL', 'VOTED', 'NOT_VOTED')),
    FOREIGN KEY ("tenant_id")
        REFERENCES "sequent_backend"."tenant" ("id")
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    FOREIGN KEY ("tenant_id", "election_event_id")
        REFERENCES "sequent_backend"."election_event" ("tenant_id", "id")
        ON UPDATE RESTRICT
        ON DELETE CASCADE
);

-- Each event's recounts: when one was asked for, started and finished, when
-- the cached counts were counted, and why the last one didn't count (it
-- timed out or failed; the previous counts stay).
CREATE TABLE "sequent_backend"."messaging_voter_count_run"
(
    "tenant_id"         uuid        NOT NULL,
    "election_event_id" uuid        NOT NULL,
    "requested_at"      timestamptz,
    "started_at"        timestamptz,
    "finished_at"       timestamptz,
    "computed_at"       timestamptz,
    "outcome"           text,
    "reason"            text,
    PRIMARY KEY ("election_event_id"),
    CHECK ("outcome" IN ('COUNTED', 'TIMED_OUT', 'FAILED')),
    FOREIGN KEY ("tenant_id")
        REFERENCES "sequent_backend"."tenant" ("id")
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    FOREIGN KEY ("tenant_id", "election_event_id")
        REFERENCES "sequent_backend"."election_event" ("tenant_id", "id")
        ON UPDATE RESTRICT
        ON DELETE CASCADE
);

-- Monthly volumes: a tenant's attempts handed to a provider, by month.
CREATE INDEX "message_tenant_attempt_month"
    ON "sequent_backend"."message" ("tenant_id", (COALESCE("dispatched_at", "created_at")))
    WHERE "direction" = 'OUTBOUND' AND "channel" IS NOT NULL;
