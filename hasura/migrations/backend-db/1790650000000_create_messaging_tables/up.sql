-- Sending accounts. Credentials live in sequent_backend.secret; this row
-- only records when each was last replaced.
CREATE TABLE "sequent_backend"."messaging_account"
(
    "id"                uuid        NOT NULL DEFAULT gen_random_uuid(),
    "tenant_id"         uuid        NOT NULL,
    "channel"           text        NOT NULL,
    "provider"          text        NOT NULL,
    "name"              text        NOT NULL,
    "sender"            jsonb       NOT NULL DEFAULT '{}'::jsonb,
    "credentials"       jsonb       NOT NULL DEFAULT '{}'::jsonb,
    "limits"            jsonb       NOT NULL DEFAULT '{}'::jsonb,
    "provider_approval" text        NOT NULL DEFAULT 'PENDING',
    "status"            jsonb       NOT NULL DEFAULT '{}'::jsonb,
    "webhook_key"       text        NOT NULL,
    "is_default"        boolean     NOT NULL DEFAULT false,
    "created_at"        timestamptz NOT NULL DEFAULT now(),
    "updated_at"        timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY ("id"),
    UNIQUE ("tenant_id", "id"),
    UNIQUE ("webhook_key"),
    FOREIGN KEY ("tenant_id")
        REFERENCES "sequent_backend"."tenant" ("id")
        ON UPDATE CASCADE
        ON DELETE CASCADE
);

CREATE UNIQUE INDEX "messaging_account_one_default_per_channel"
    ON "sequent_backend"."messaging_account" ("tenant_id", "channel")
    WHERE "is_default";

-- One row per delivery attempt, inbound message or provider event. Never
-- holds message bodies or codes.
CREATE TABLE "sequent_backend"."message"
(
    "id"                    uuid        NOT NULL DEFAULT gen_random_uuid(),
    "tenant_id"             uuid        NOT NULL,
    "election_event_id"     uuid,
    "voter_id"              text,
    "account_id"            uuid,
    "channel"               text        NOT NULL,
    "direction"             text        NOT NULL,
    "purpose"               text,
    "template_alias"        text,
    "language"              text,
    "masked_destination"    text        NOT NULL,
    "destination_digest"    text        NOT NULL,
    "destination_country"   text,
    "logical_key"           text,
    "attempt"               integer     NOT NULL DEFAULT 1,
    "state"                 text        NOT NULL,
    "provider_message_id"   text,
    "error"                 text,
    "billing"               jsonb,
    "created_at"            timestamptz NOT NULL DEFAULT now(),
    "updated_at"            timestamptz NOT NULL DEFAULT now(),
    "accepted_at"           timestamptz,
    "delivered_at"          timestamptz,
    "failed_at"             timestamptz,
    PRIMARY KEY ("id"),
    UNIQUE ("tenant_id", "logical_key", "attempt"),
    FOREIGN KEY ("tenant_id")
        REFERENCES "sequent_backend"."tenant" ("id")
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    FOREIGN KEY ("tenant_id", "election_event_id")
        REFERENCES "sequent_backend"."election_event" ("tenant_id", "id")
        ON UPDATE RESTRICT
        ON DELETE CASCADE,
    FOREIGN KEY ("account_id")
        REFERENCES "sequent_backend"."messaging_account" ("id")
        ON UPDATE CASCADE
        ON DELETE SET NULL
);

CREATE INDEX "message_provider_message_id"
    ON "sequent_backend"."message" ("account_id", "provider_message_id");
CREATE INDEX "message_event_voter"
    ON "sequent_backend"."message" ("tenant_id", "election_event_id", "voter_id", "created_at");
CREATE INDEX "message_unresolved"
    ON "sequent_backend"."message" ("tenant_id", "state")
    WHERE "state" IN ('QUEUED', 'UNKNOWN');

-- One-time references binding a Messenger conversation to an
-- authentication session and its live code. The reference itself is never
-- stored, only its digest; the code is encrypted and removed on use,
-- replacement or expiry.
CREATE TABLE "sequent_backend"."messenger_link"
(
    "id"                    uuid        NOT NULL DEFAULT gen_random_uuid(),
    "tenant_id"             uuid        NOT NULL,
    "election_event_id"     uuid,
    "account_id"            uuid        NOT NULL,
    "reference_digest"      text        NOT NULL,
    "link_word_digest"      text        NOT NULL,
    "auth_session_digest"   text        NOT NULL,
    "challenge_digest"      text        NOT NULL,
    "encrypted_payload"     bytea,
    "language"              text,
    "state"                 text        NOT NULL,
    "page_scoped_id"        text,
    "message_id"            uuid,
    "expires_at"            timestamptz NOT NULL,
    "created_at"            timestamptz NOT NULL DEFAULT now(),
    "updated_at"            timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY ("id"),
    UNIQUE ("reference_digest"),
    UNIQUE ("account_id", "link_word_digest"),
    FOREIGN KEY ("tenant_id")
        REFERENCES "sequent_backend"."tenant" ("id")
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    FOREIGN KEY ("account_id")
        REFERENCES "sequent_backend"."messaging_account" ("id")
        ON UPDATE CASCADE
        ON DELETE CASCADE
);

CREATE INDEX "messenger_link_session"
    ON "sequent_backend"."messenger_link" ("tenant_id", "auth_session_digest");
