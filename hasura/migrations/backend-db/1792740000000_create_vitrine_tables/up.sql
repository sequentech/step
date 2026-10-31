-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

CREATE TABLE "sequent_backend"."vitrine_log"
(
    "tenant_id"         uuid        NOT NULL,
    "election_event_id" uuid        NOT NULL,
    "name"              text        NOT NULL CHECK ("name" IN ('ballots', 'events')),
    "sidecar_name"      text        NOT NULL UNIQUE,
    "log_id"            bigint      NOT NULL,
    "instance_id"       text        NOT NULL,
    "origin"            text        NOT NULL,
    "url"               text        NOT NULL,
    "vkey"              text        NOT NULL,
    "log_time_vkey"     text        NOT NULL,
    "visibility"        text        NOT NULL CHECK ("visibility" IN ('public', 'private')),
    "submission_token"  text,
    "policy_ack"        text,
    "enrolment"         text,
    "frozen_at"         timestamptz,
    "created_at"        timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY ("tenant_id", "election_event_id", "name"),
    FOREIGN KEY ("tenant_id")
        REFERENCES "sequent_backend"."tenant" ("id")
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    FOREIGN KEY ("tenant_id", "election_event_id")
        REFERENCES "sequent_backend"."election_event" ("tenant_id", "id")
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    CONSTRAINT "vitrine_log_submission_token_check"
        CHECK (("name" = 'events') = ("submission_token" IS NOT NULL))
);
COMMENT ON TABLE "sequent_backend"."vitrine_log" IS
    'The Vitrine logs of an election event, as the sidecar created them. It holds the events log''s submission token, so Hasura does not track it: only Harvest and windmill read it.';
COMMENT ON COLUMN "sequent_backend"."vitrine_log"."name" IS
    'The log''s name in the event''s policy.';
COMMENT ON COLUMN "sequent_backend"."vitrine_log"."sidecar_name" IS
    'The log''s name on the sidecar, the path under its base URL.';

CREATE TABLE "sequent_backend"."vitrine_event"
(
    "tenant_id"          uuid        NOT NULL,
    "election_event_id"  uuid        NOT NULL,
    "policy_salt"        text        CHECK ("policy_salt" ~ '^[0-9a-f]{64}$'),
    "policy"             text,
    "policy_id"          text        CHECK ("policy_id" ~ '^sha256:[0-9a-f]{64}$'),
    "policy_uploaded_at" timestamptz,
    "created_at"         timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY ("tenant_id", "election_event_id"),
    FOREIGN KEY ("tenant_id")
        REFERENCES "sequent_backend"."tenant" ("id")
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    FOREIGN KEY ("tenant_id", "election_event_id")
        REFERENCES "sequent_backend"."election_event" ("tenant_id", "id")
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    CONSTRAINT "vitrine_event_policy_check"
        CHECK (("policy" IS NULL) = ("policy_id" IS NULL)
            AND ("policy" IS NULL) = ("policy_uploaded_at" IS NULL))
);
COMMENT ON TABLE "sequent_backend"."vitrine_event" IS
    'The trust policy of an election event with transparency: the signed policy the Board sealed, once uploaded, and the salt of a private event''s draft.';

CREATE TABLE "sequent_backend"."vitrine_statement"
(
    "id"                uuid        NOT NULL DEFAULT gen_random_uuid(),
    "tenant_id"         uuid        NOT NULL,
    "election_event_id" uuid        NOT NULL,
    "key"               text        NOT NULL,
    "kind"              text        NOT NULL CHECK ("kind" IN ('config', 'milestone')),
    "election_id"       uuid,
    "statement"         text        NOT NULL,
    "entry"             text        NOT NULL,
    "leaf_hash"         text        NOT NULL CHECK ("leaf_hash" ~ '^[0-9a-f]{64}$'),
    "style_hashes"      text[],
    "chain"             text,
    "created_at"        timestamptz NOT NULL DEFAULT now(),
    "receipted_at"      timestamptz,
    PRIMARY KEY ("id"),
    FOREIGN KEY ("tenant_id")
        REFERENCES "sequent_backend"."tenant" ("id")
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    FOREIGN KEY ("tenant_id", "election_event_id")
        REFERENCES "sequent_backend"."election_event" ("tenant_id", "id")
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    CONSTRAINT "vitrine_statement_key_key"
        UNIQUE ("tenant_id", "election_event_id", "key"),
    CONSTRAINT "vitrine_statement_config_check"
        CHECK (("kind" = 'config') = ("election_id" IS NOT NULL)
            AND ("kind" = 'config') = ("style_hashes" IS NOT NULL)),
    CONSTRAINT "vitrine_statement_chain_check"
        CHECK (("chain" IS NULL) = ("receipted_at" IS NULL))
);
COMMENT ON TABLE "sequent_backend"."vitrine_statement" IS
    'Statements windmill appended to an election event''s Vitrine events log, with their entries and, once a checkpoint that meets the quorum covers them, their receipt chains.';
COMMENT ON COLUMN "sequent_backend"."vitrine_statement"."key" IS
    'What the statement is, once per event: config:<election id>, voting-opened, voting-closed, tally-completed:<tally session id> or results-published:<publication id>.';
COMMENT ON COLUMN "sequent_backend"."vitrine_statement"."style_hashes" IS
    'Config statements: the hex SHA-256 of each distinct ballot style of the election, sorted, which its styles line hashes.';

CREATE OR REPLACE FUNCTION "sequent_backend"."vitrine_status_opened"(status jsonb)
RETURNS boolean
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT coalesce(bool_or(
        coalesce(status ->> (channel || 'voting_status'), 'NOT_STARTED') <> 'NOT_STARTED'
        OR status -> (channel || 'voting_period_dates') ->> 'first_started_at' IS NOT NULL
    ), false)
    FROM unnest(ARRAY['', 'kiosk_', 'early_', 'telephone_']) AS channel;
$$;
COMMENT ON FUNCTION "sequent_backend"."vitrine_status_opened"(jsonb) IS
    'Whether an election event''s or an election''s status shows that voting opened on any channel.';

CREATE OR REPLACE FUNCTION "sequent_backend"."vitrine_voting_opened"(tenant_id uuid, election_event_id uuid)
RETURNS boolean
LANGUAGE sql
STABLE
AS $$
    SELECT EXISTS (
        SELECT 1
        FROM "sequent_backend"."election_event" e
        WHERE e.tenant_id = $1
          AND e.id = $2
          AND "sequent_backend"."vitrine_status_opened"(e.status)
    ) OR EXISTS (
        SELECT 1
        FROM "sequent_backend"."election" el
        WHERE el.tenant_id = $1
          AND el.election_event_id = $2
          AND "sequent_backend"."vitrine_status_opened"(el.status)
    ) OR EXISTS (
        SELECT 1
        FROM "sequent_backend"."vitrine_statement" s
        WHERE s.tenant_id = $1
          AND s.election_event_id = $2
          AND s.key = 'voting-opened'
    );
$$;
COMMENT ON FUNCTION "sequent_backend"."vitrine_voting_opened"(uuid, uuid) IS
    'Whether voting opened in an election event: its one voting window started, so its transparency mode and policy are locked.';
