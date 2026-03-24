ALTER TABLE "sequent_backend"."certificate_authority"
    DROP COLUMN "election_event_id";

ALTER TABLE "sequent_backend"."certificate_authority"
    DROP CONSTRAINT IF EXISTS "certificate_authority_tenant_id_election_event_id_fingerprint_s";

ALTER TABLE "sequent_backend"."certificate_authority"
    ADD CONSTRAINT "certificate_authority_tenant_id_fingerprint_sha256_key"
    UNIQUE ("tenant_id", "fingerprint_sha256");

CREATE INDEX ON "sequent_backend"."certificate_authority" ("tenant_id");
