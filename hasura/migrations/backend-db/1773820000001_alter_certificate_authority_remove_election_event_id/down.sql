-- Note: this migration cannot be fully reversed when rows exist, because
-- election_event_id was NOT NULL and existing rows would have no value.
ALTER TABLE "sequent_backend"."certificate_authority"
    DROP CONSTRAINT "certificate_authority_tenant_id_fingerprint_sha256_key";

ALTER TABLE "sequent_backend"."certificate_authority"
    ADD COLUMN "election_event_id" uuid;

ALTER TABLE "sequent_backend"."certificate_authority"
    ADD CONSTRAINT "certificate_authority_election_event_id_fkey"
        FOREIGN KEY ("election_event_id")
            REFERENCES "sequent_backend"."election_event"("id") ON DELETE CASCADE;

ALTER TABLE "sequent_backend"."certificate_authority"
    ADD CONSTRAINT "certificate_authority_tenant_id_election_event_id_fingerprint_sha256_key"
        UNIQUE ("tenant_id", "election_event_id", "fingerprint_sha256");

CREATE INDEX ON "sequent_backend"."certificate_authority" ("tenant_id", "election_event_id");

-- Set NOT NULL only once values have been backfilled.
-- ALTER TABLE "sequent_backend"."certificate_authority"
--     ALTER COLUMN "election_event_id" SET NOT NULL;
