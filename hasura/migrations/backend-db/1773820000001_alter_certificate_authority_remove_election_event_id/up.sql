-- Drop the foreign key first, then the column (which auto-drops the unique
-- constraint and index that included election_event_id).
ALTER TABLE "sequent_backend"."certificate_authority"
    DROP CONSTRAINT "certificate_authority_election_event_id_fkey";

ALTER TABLE "sequent_backend"."certificate_authority"
    DROP COLUMN "election_event_id";

-- New unique constraint: one cert per tenant (fingerprint is globally unique
-- for a given tenant since only the super-tenant imports CAs).
ALTER TABLE "sequent_backend"."certificate_authority"
    ADD CONSTRAINT "certificate_authority_tenant_id_fingerprint_sha256_key"
        UNIQUE ("tenant_id", "fingerprint_sha256");

CREATE INDEX ON "sequent_backend"."certificate_authority" ("tenant_id");
