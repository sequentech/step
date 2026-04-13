-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

ALTER TABLE "sequent_backend"."certificate_authority"
    DROP CONSTRAINT IF EXISTS "certificate_authority_fingerprint_sha256_key";

ALTER TABLE "sequent_backend"."certificate_authority"
    ADD COLUMN "tenant_id"          uuid NOT NULL DEFAULT gen_random_uuid(),
    ADD COLUMN "election_event_id"  uuid NOT NULL DEFAULT gen_random_uuid();

ALTER TABLE "sequent_backend"."certificate_authority"
    ADD CONSTRAINT "certificate_authority_tenant_id_election_event_id_fingerprint_s"
    UNIQUE ("tenant_id", "election_event_id", "fingerprint_sha256");

CREATE INDEX ON "sequent_backend"."certificate_authority" ("tenant_id", "election_event_id");
