-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- This pool logs in directly over the app socket. Include inherited, PUBLIC
-- and column grants when checking what its sessions can actually do.
WITH reads(signature) AS (VALUES
    ('sequent_backend.operational_initialize_get_ca_enrollment(uuid,uuid,uuid,text,uuid,uuid,text)'::regprocedure),
    ('sequent_backend.operational_initialize_ca_version(uuid,uuid,text,text)'::regprocedure),
    ('sequent_backend.operational_signing_local_initialize_installation_id(uuid,uuid,uuid,uuid,text)'::regprocedure),
    ('sequent_backend.felix3_initialize_ca_request_matches(uuid,uuid,uuid,uuid,uuid,uuid,uuid,uuid,uuid)'::regprocedure)
)
SELECT current_user = session_user AND current_user::text = $1::text
   AND current_database()::text = $2::text AND inet_client_addr() IS NULL
   AND pg_has_role(current_user,'sequent_os_initialize_ca','USAGE')
   AND EXISTS (SELECT FROM pg_roles WHERE rolname = current_user AND rolcanlogin
               AND NOT (rolsuper OR rolcreatedb OR rolcreaterole OR rolreplication OR rolbypassrls))
   AND EXISTS (SELECT FROM pg_roles WHERE rolname = 'sequent_os_initialize_ca' AND NOT rolcanlogin
               AND NOT (rolsuper OR rolcreatedb OR rolcreaterole OR rolreplication OR rolbypassrls))
   AND NOT EXISTS (SELECT FROM pg_roles r WHERE r.rolname::text NOT IN ($1,'sequent_os_initialize_ca')
                   AND pg_has_role(current_user,r.oid,'MEMBER'))
   AND NOT EXISTS (SELECT FROM pg_auth_members WHERE member = current_user::regrole AND admin_option)
   AND NOT has_database_privilege(current_user,current_database(),'CREATE')
   AND has_schema_privilege(current_user,'sequent_backend','USAGE')
   AND NOT has_schema_privilege(current_user,'sequent_backend','CREATE')
   AND NOT EXISTS (SELECT FROM reads WHERE NOT has_function_privilege(current_user,signature,'EXECUTE'))
   AND NOT EXISTS (
       SELECT FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
       WHERE n.nspname = 'sequent_backend' AND CASE
           WHEN c.relkind = 'S' THEN has_sequence_privilege(current_user,c.oid,'USAGE,SELECT,UPDATE')
           WHEN c.relkind IN ('r','p','v','m','f') THEN
               has_table_privilege(current_user,c.oid,'SELECT,INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER')
               OR has_any_column_privilege(current_user,c.oid,'SELECT,INSERT,UPDATE,REFERENCES')
               OR CASE WHEN current_setting('server_version_num')::integer >= 170000
                       THEN has_table_privilege(current_user,c.oid,'MAINTAIN') ELSE false END
           ELSE false END)
   AND NOT EXISTS (
       SELECT FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace
       WHERE n.nspname = 'sequent_backend' AND p.prosecdef
         AND p.prorettype NOT IN ('trigger'::regtype,'event_trigger'::regtype)
         AND has_function_privilege(current_user,p.oid,'EXECUTE')
         AND p.oid NOT IN (SELECT signature FROM reads)
         -- The original CA group also inherits the read-only sealed marker.
         AND (p.oid IS DISTINCT FROM to_regprocedure('sequent_backend.operational_signing_demo_fixture_id(uuid,uuid,text,text)')::oid
              OR p.provolatile <> 's'))
