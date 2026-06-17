-- Per-ceremony trustee key storage. One row per (trustee, election_event,
-- keys_ceremony) so a trustee reused across ceremonies has an independent,
-- non-overwriting key row per ceremony.
CREATE TABLE "sequent_backend"."trustee_ceremony_key" (
    "id" uuid NOT NULL DEFAULT gen_random_uuid(),
    "tenant_id" uuid NOT NULL,
    "trustee_id" uuid NOT NULL,
    "election_event_id" uuid NOT NULL,
    "keys_ceremony_id" uuid NOT NULL,
    "public_key" text NOT NULL,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    "last_updated_at" timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY ("id"),
    FOREIGN KEY ("tenant_id") REFERENCES "sequent_backend"."tenant"("id") ON UPDATE restrict ON DELETE restrict,
    FOREIGN KEY ("trustee_id") REFERENCES "sequent_backend"."trustee"("id") ON UPDATE restrict ON DELETE cascade,
    FOREIGN KEY ("election_event_id") REFERENCES "sequent_backend"."election_event"("id") ON UPDATE restrict ON DELETE cascade,
    -- keys_ceremony has a composite primary key (id, tenant_id, election_event_id),
    -- so the FK must reference all three columns.
    FOREIGN KEY ("keys_ceremony_id", "tenant_id", "election_event_id")
        REFERENCES "sequent_backend"."keys_ceremony"("id", "tenant_id", "election_event_id")
        ON UPDATE restrict ON DELETE cascade,
    UNIQUE ("tenant_id", "trustee_id", "election_event_id", "keys_ceremony_id")
);
