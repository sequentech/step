CREATE TABLE "sequent_backend"."certificate_authority" (
    "id"                 uuid        NOT NULL DEFAULT gen_random_uuid(),
    "tenant_id"          uuid        NOT NULL,
    "election_event_id"  uuid        NOT NULL,
    "common_name"        text        NOT NULL,
    "subject"            text        NOT NULL,
    "issuer_common_name" text        NOT NULL,
    "issuer"             text        NOT NULL,
    "not_before"         timestamptz NOT NULL,
    "not_after"          timestamptz NOT NULL,
    "fingerprint_sha256" text        NOT NULL,
    "serial_number"      text        NOT NULL,
    "pem"                text        NOT NULL,
    "created_at"         timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY ("id"),
    UNIQUE ("tenant_id", "election_event_id", "fingerprint_sha256"),
    FOREIGN KEY ("election_event_id")
        REFERENCES "sequent_backend"."election_event"("id") ON DELETE CASCADE
);
CREATE INDEX ON "sequent_backend"."certificate_authority" ("tenant_id", "election_event_id");
