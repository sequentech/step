-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- What the tenant selection page may learn about one tenant before sign-in:
-- its id, to open the tenant's realm, and its logo and style.
CREATE VIEW sequent_backend.sign_in_tenant AS
SELECT id, jsonb_strip_nulls(jsonb_build_object(
           'logo_url', annotations -> 'logo_url', 'css', annotations -> 'css')) AS annotations
FROM sequent_backend.tenant;

-- At most one tenant, named by its slug or id; both must match when both are given.
CREATE FUNCTION sequent_backend.tenant_for_sign_in(slug text DEFAULT NULL, id uuid DEFAULT NULL)
RETURNS SETOF sequent_backend.sign_in_tenant LANGUAGE sql STABLE AS $$
    SELECT s.* FROM sequent_backend.tenant t
    JOIN sequent_backend.sign_in_tenant s ON s.id = t.id
    WHERE (tenant_for_sign_in.slug IS NOT NULL OR tenant_for_sign_in.id IS NOT NULL)
      AND (tenant_for_sign_in.slug IS NULL OR t.slug = tenant_for_sign_in.slug)
      AND (tenant_for_sign_in.id IS NULL OR t.id = tenant_for_sign_in.id)
    LIMIT 1;
$$;
