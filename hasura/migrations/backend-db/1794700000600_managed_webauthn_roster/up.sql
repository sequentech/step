-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Local receiver authority: not tracked in Hasura or part of tenant package
-- imports, exports, or restored tenant settings.
CREATE TABLE sequent_backend.managed_webauthn_roster (
    tenant_id uuid PRIMARY KEY REFERENCES sequent_backend.tenant(id) ON DELETE CASCADE,
    roster_json text NOT NULL CHECK (octet_length(roster_json) BETWEEN 1 AND 262144),
    sha256 text NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    generation bigint NOT NULL CHECK (generation > 0),
    activated_by text NOT NULL CHECK (octet_length(activated_by) BETWEEN 1 AND 255),
    activated_at timestamptz NOT NULL DEFAULT now()
);
REVOKE ALL ON sequent_backend.managed_webauthn_roster FROM PUBLIC;
