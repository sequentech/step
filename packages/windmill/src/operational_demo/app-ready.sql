-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
SELECT current_user=session_user AND current_user='os14990_windmill'
            AND current_database()='operational_demo_14990' AND inet_client_addr() IS NULL
            AND EXISTS (SELECT FROM pg_roles r WHERE r.rolname=current_user
                AND NOT (rolsuper OR rolcreaterole OR rolcreatedb OR rolreplication OR rolbypassrls))
            AND pg_has_role(current_user,'sequent_os_approve','USAGE')
            AND NOT EXISTS (SELECT FROM pg_roles r
                WHERE starts_with(r.rolname, 'sequent_os_') AND r.rolname <> 'sequent_os_approve'
                AND pg_has_role(current_user,r.oid,'MEMBER'))
            AND NOT has_schema_privilege(current_user,'sequent_backend','CREATE')
            AND NOT has_any_column_privilege(current_user,'sequent_backend.operational_signing_intent','INSERT,UPDATE,REFERENCES')
            AND NOT has_table_privilege(current_user,'sequent_backend.operational_signing_intent','DELETE,TRUNCATE,TRIGGER')
