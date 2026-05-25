CREATE TABLE "sequent_backend"."phone_blacklist"
(
    "id"                uuid        NOT NULL DEFAULT gen_random_uuid(),
    "tenant_id"         uuid        NOT NULL,
    "election_event_id" uuid,                                                  -- Null for tenant-wide blacklisting.
    "phone_e164"        text        NOT NULL,
    "reason"            text,
    "created_at"        timestamptz NOT NULL DEFAULT now(),
    "created_by"        uuid        NOT NULL,
    PRIMARY KEY ("id"),
    FOREIGN KEY ("tenant_id")
        REFERENCES "sequent_backend"."tenant" ("id")
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    FOREIGN KEY ("tenant_id", "election_event_id")
        REFERENCES "sequent_backend"."election_event" ("tenant_id", "id")
        ON UPDATE RESTRICT
        ON DELETE RESTRICT,
    UNIQUE NULLS NOT DISTINCT ("tenant_id", "election_event_id", "phone_e164") -- Null election_event_id means tenant-wide, prevent duplicates
);
CREATE EXTENSION IF NOT EXISTS pgcrypto;
