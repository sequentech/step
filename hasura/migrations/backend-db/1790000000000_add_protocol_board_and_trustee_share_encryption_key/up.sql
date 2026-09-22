ALTER TABLE "sequent_backend"."trustee"
    ADD COLUMN "share_encryption_public_key" text;

CREATE TABLE "sequent_backend"."protocol_board" (
    "id" uuid NOT NULL DEFAULT gen_random_uuid(),
    "tenant_id" uuid NOT NULL,
    "election_event_id" uuid NOT NULL,
    "name" text NOT NULL,
    "kind" text NOT NULL,
    "lifecycle" text NOT NULL DEFAULT 'ACTIVE',
    "parent_id" uuid,
    "keys_ceremony_id" uuid,
    "configuration_hash" text NOT NULL,
    "trustee_ids" uuid[] NOT NULL,
    "trustee_reports" jsonb,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    "last_updated_at" timestamptz NOT NULL DEFAULT now(),
    "annotations" jsonb,
    PRIMARY KEY ("id"),
    UNIQUE ("name"),
    FOREIGN KEY ("tenant_id") REFERENCES "sequent_backend"."tenant"("id") ON UPDATE restrict ON DELETE cascade,
    FOREIGN KEY ("election_event_id") REFERENCES "sequent_backend"."election_event"("id") ON UPDATE restrict ON DELETE cascade,
    FOREIGN KEY ("parent_id") REFERENCES "sequent_backend"."protocol_board"("id") ON UPDATE restrict ON DELETE cascade
);

CREATE INDEX "protocol_board_keys_ceremony_id_idx"
    ON "sequent_backend"."protocol_board" ("keys_ceremony_id");
