-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

-- Internal server registry: deliberately not tracked by Hasura metadata.
CREATE TABLE sequent_backend.configuration_package_staging (
    tenant_id uuid NOT NULL REFERENCES sequent_backend.tenant (id) ON DELETE CASCADE,
    document_id uuid NOT NULL REFERENCES sequent_backend.document (id) ON DELETE CASCADE,
    storage_id uuid NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    PRIMARY KEY (tenant_id, document_id),
    UNIQUE (document_id)
);
