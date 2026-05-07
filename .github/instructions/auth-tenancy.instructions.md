---
applyTo: "packages/harvest/**/*.rs,packages/keycloak-extensions/**/*.java,packages/admin-portal/**/*.ts,packages/admin-portal/**/*.tsx,packages/voting-portal/**/*.ts,packages/voting-portal/**/*.tsx,packages/ui-core/**/*.ts,packages/ui-core/**/*.tsx,packages/ui-essentials/**/*.ts,packages/ui-essentials/**/*.tsx,hasura/metadata/**/*.yaml,hasura/migrations/**/*.sql"
---
<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Authorization And Tenant Boundary Review

- Prioritize tenant isolation, election-event isolation, and authorization correctness over UI polish or code style.
- Check that server-side access control remains enforced. Hiding a button or route in the frontend is not sufficient protection.
- Review Keycloak and Hasura changes for privilege escalation, missing role checks, cross-tenant reads or writes, and accidental widening of default permissions.
- For admin and voting portals, verify admin-only actions remain protected, contest state stays accurate, and tenant or event identifiers in routes, storage, document URLs, and API variables cannot leak data across tenants.
- Rich text or HTML must stay sanitized through existing safe paths. Flag new raw HTML rendering or changes that bypass sanitization.
- For user, role, and permission changes, verify compatibility with the default tenant template and existing realms.
- For logs, exports, or public document access, check that voter-linked or tenant-linked data is not exposed to broader audiences.
- Expect tests for authorization failures, wrong-tenant access, sanitized rich text, and permission-gated frontend states.
