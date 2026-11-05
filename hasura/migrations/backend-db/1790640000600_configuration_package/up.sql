-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Signed configuration packages a tenant imported. Each was verified before
-- anything in it was read; the newest revision per configuration is what a
-- later import must be newer than, and the manifest is what publication and
-- report generation compare against.
CREATE TABLE sequent_backend.configuration_package (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL,
    -- The election event the import created.
    election_event_id uuid NOT NULL,
    -- The configuration's external id, as the manifest names it.
    external_id text NOT NULL,
    revision bigint NOT NULL,
    manifest_sha256 text NOT NULL,
    manifest jsonb NOT NULL,
    signer_subject text NOT NULL,
    signer_serial text NOT NULL,
    signer_fingerprint text NOT NULL,
    approvers jsonb NOT NULL,
    imported_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    CONSTRAINT configuration_package_revision_positive CHECK (revision > 0),
    CONSTRAINT configuration_package_manifest_sha256 CHECK (manifest_sha256 ~ '^[0-9a-f]{64}$'),
    CONSTRAINT configuration_package_revision_once UNIQUE (tenant_id, external_id, revision)
);

CREATE INDEX configuration_package_event
    ON sequent_backend.configuration_package (tenant_id, election_event_id);

-- Revocation lists a tenant has seen, from packages or its settings. A key
-- revoked by any of them signs nothing that imports from then on.
CREATE TABLE sequent_backend.configuration_revocation_list (
    tenant_id uuid NOT NULL,
    sha256 text NOT NULL,
    der bytea NOT NULL,
    seen_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (tenant_id, sha256)
);
