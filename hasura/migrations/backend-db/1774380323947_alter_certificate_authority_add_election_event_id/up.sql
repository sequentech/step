ALTER TABLE "sequent_backend"."certificate_authority"
    ADD COLUMN "election_event_id" uuid NOT NULL
        REFERENCES "sequent_backend"."election_event"("id") ON DELETE CASCADE;

ALTER TABLE "sequent_backend"."certificate_authority"
    DROP CONSTRAINT "certificate_authority_tenant_id_fingerprint_sha256_key";

ALTER TABLE "sequent_backend"."certificate_authority"
    ADD CONSTRAINT "certificate_authority_tenant_id_election_event_id_fingerprint_s"
    UNIQUE ("tenant_id", "election_event_id", "fingerprint_sha256");

DROP INDEX "sequent_backend"."certificate_authority_tenant_id_idx";

CREATE INDEX ON "sequent_backend"."certificate_authority" ("tenant_id", "election_event_id");
