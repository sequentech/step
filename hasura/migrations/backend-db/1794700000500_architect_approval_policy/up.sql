-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

CREATE TABLE sequent_backend.architect_approval_policy (
    tenant_id uuid PRIMARY KEY REFERENCES sequent_backend.tenant(id) ON DELETE CASCADE,
    required_approvals smallint CHECK (required_approvals BETWEEN 1 AND 16),
    generation bigint NOT NULL CHECK (generation > 0)
);
CREATE TABLE sequent_backend.architect_approval_policy_reader_binding (
    role_name name PRIMARY KEY,
    tenant_id uuid NOT NULL REFERENCES sequent_backend.tenant(id) ON DELETE CASCADE
);
CREATE VIEW sequent_backend.architect_approval_policy_reader WITH (security_barrier = true) AS
SELECT tenant.id AS tenant_id, COALESCE(policy.required_approvals, 2::smallint) AS required_approvals,
       COALESCE(policy.generation, 0::bigint) AS generation
FROM sequent_backend.architect_approval_policy_reader_binding AS binding
JOIN sequent_backend.tenant AS tenant ON tenant.id = binding.tenant_id
LEFT JOIN sequent_backend.architect_approval_policy AS policy ON policy.tenant_id = tenant.id
WHERE binding.role_name = session_user;
REVOKE ALL ON sequent_backend.architect_approval_policy,
    sequent_backend.architect_approval_policy_reader_binding,
    sequent_backend.architect_approval_policy_reader FROM PUBLIC;
