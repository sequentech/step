-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

-- Drop the FK constraint and index that depend on the columns being removed,
-- then drop the columns themselves and replace the unique constraint.

ALTER TABLE "sequent_backend"."certificate_authority"
    DROP CONSTRAINT IF EXISTS "certificate_authority_election_event_id_fkey";

DROP INDEX IF EXISTS "sequent_backend"."certificate_authority_tenant_id_election_event_id_idx";

ALTER TABLE "sequent_backend"."certificate_authority"
    DROP CONSTRAINT IF EXISTS "certificate_authority_tenant_id_election_event_id_fingerprint_s";

ALTER TABLE "sequent_backend"."certificate_authority"
    DROP COLUMN "election_event_id",
    DROP COLUMN "tenant_id";

-- Remove duplicate fingerprints, keeping the oldest row per fingerprint.
DELETE FROM "sequent_backend"."certificate_authority"
WHERE id NOT IN (
    SELECT DISTINCT ON (fingerprint_sha256) id
    FROM "sequent_backend"."certificate_authority"
    ORDER BY fingerprint_sha256, created_at ASC
);

ALTER TABLE "sequent_backend"."certificate_authority"
    ADD CONSTRAINT "certificate_authority_fingerprint_sha256_key"
    UNIQUE ("fingerprint_sha256");
