-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Owner, security mode and every non-owner privilege of the sequent_backend
-- schema, its relations, columns, functions and types, in a stable order.
-- pg_dump --no-owner --no-privileges (the CI schema diff) sees none of this.
\pset tuples_only on
\pset format unaligned
WITH acl AS (
    SELECT 'schema' AS kind, n.nspname::text AS name, n.nspowner AS owner, ''::text AS extra,
           coalesce(n.nspacl, acldefault('n', n.nspowner)) AS acl
    FROM pg_namespace n WHERE n.nspname = 'sequent_backend'
    UNION ALL
    SELECT 'relation', c.oid::regclass::text, c.relowner, c.relkind::text,
           coalesce(c.relacl, acldefault(CASE WHEN c.relkind = 'S' THEN 's' ELSE 'r' END::"char", c.relowner))
    FROM pg_class c WHERE c.relnamespace = 'sequent_backend'::regnamespace
      AND c.relkind IN ('r', 'v', 'm', 'S', 'p', 'f')
    UNION ALL
    SELECT 'column', a.attrelid::regclass::text || '.' || a.attname, c.relowner, '', a.attacl
    FROM pg_attribute a JOIN pg_class c ON c.oid = a.attrelid
    WHERE c.relnamespace = 'sequent_backend'::regnamespace AND a.attacl IS NOT NULL
    UNION ALL
    SELECT 'function', p.oid::regprocedure::text, p.proowner,
           CASE WHEN p.prosecdef THEN 'definer' ELSE 'invoker' END || ' ' || coalesce(p.proconfig::text, ''),
           coalesce(p.proacl, acldefault('f', p.proowner))
    FROM pg_proc p WHERE p.pronamespace = 'sequent_backend'::regnamespace
    UNION ALL
    SELECT 'type', t.oid::regtype::text, t.typowner, '', coalesce(t.typacl, acldefault('T', t.typowner))
    FROM pg_type t WHERE t.typnamespace = 'sequent_backend'::regnamespace AND t.typtype IN ('c', 'e', 'd')
      AND NOT EXISTS (SELECT FROM pg_class c WHERE c.reltype = t.oid AND c.relkind <> 'c')
)
SELECT kind, name, owner::regrole::text, extra,
       coalesce((SELECT string_agg(CASE WHEN x.grantee = 0 THEN 'PUBLIC' ELSE x.grantee::regrole::text END
                                   || ':' || x.privilege_type, ',' ORDER BY x.grantee, x.privilege_type)
                 FROM aclexplode(acl.acl) x WHERE x.grantee <> acl.owner), '')
FROM acl ORDER BY kind, name;
