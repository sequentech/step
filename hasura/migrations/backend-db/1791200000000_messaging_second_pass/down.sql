-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- The deleted inbound messages can't be restored. The event
-- configurations go back to the election event annotation, which every
-- save since this release also writes (R7-25). Each rewrite says what it
-- changed with RAISE NOTICE, so the rollback's output records it.
--
-- Run this before deploying the old binaries: they can't read message
-- rows without a channel.

DO $$
DECLARE
    configurations bigint;
BEGIN
    SELECT count(*) INTO configurations FROM "sequent_backend"."messaging_event_config";
    RAISE NOTICE 'messaging rollback: % event configuration(s) dropped; the election event annotation holds each one''s last save', configurations;
END $$;

DROP INDEX "sequent_backend"."message_tenant_attempt_month";
DROP TABLE "sequent_backend"."messaging_voter_count_run";
DROP TABLE "sequent_backend"."messaging_voter_count";

DO $$
DECLARE
    links bigint;
BEGIN
    UPDATE "sequent_backend"."messenger_link"
        SET "link_word_digest" = "reference_digest"
        WHERE "link_word_digest" IS NULL;
    GET DIAGNOSTICS links = ROW_COUNT;
    RAISE NOTICE 'messaging rollback: % Messenger link(s) without a link word given their reference digest in its place', links;
END $$;
ALTER TABLE "sequent_backend"."messenger_link"
    ALTER COLUMN "link_word_digest" SET NOT NULL;

DROP VIEW "sequent_backend"."message_latest_attempt";
DROP INDEX "sequent_backend"."message_unlogged";

-- The first release needs a channel: rows without one go back on email,
-- the channel such a voter's legacy rule gives.
DO $$
DECLARE
    relabelled bigint;
BEGIN
    UPDATE "sequent_backend"."message" SET "channel" = 'EMAIL' WHERE "channel" IS NULL;
    GET DIAGNOSTICS relabelled = ROW_COUNT;
    RAISE NOTICE 'messaging rollback: % message(s) without a channel relabelled EMAIL', relabelled;
END $$;
ALTER TABLE "sequent_backend"."message"
    ALTER COLUMN "channel" SET NOT NULL;

DROP INDEX "sequent_backend"."message_unresolved_keyset";
DROP INDEX "sequent_backend"."message_retry_due";

ALTER TABLE "sequent_backend"."message"
    DROP COLUMN "logged_at",
    DROP COLUMN "billable",
    DROP COLUMN "billing_category",
    DROP COLUMN "last_reconciled_at",
    DROP COLUMN "dispatched_at",
    DROP COLUMN "next_retry_at";

DROP TABLE "sequent_backend"."messenger_contact";
DROP TABLE "sequent_backend"."messaging_rate_bucket";
DROP TABLE "sequent_backend"."messaging_event_config";
