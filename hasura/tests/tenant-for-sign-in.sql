-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-- Run with psql -v ON_ERROR_STOP=1 -f this-file.sql against a disposable empty database.
BEGIN;
CREATE SCHEMA sequent_backend;
CREATE TABLE sequent_backend.tenant (id uuid PRIMARY KEY, slug text UNIQUE NOT NULL, annotations jsonb, settings jsonb, is_active boolean);
\ir ../migrations/backend-db/1793020020000_tenant_for_sign_in/up.sql
DO $$
DECLARE
    council uuid := '00000000-0000-4000-8000-000000000001';
    board uuid := '00000000-0000-4000-8000-000000000002';
    look jsonb := '{"logo_url": "https://logo.test/council.svg", "css": "h1 { letter-spacing: 3px; }"}';
BEGIN
    INSERT INTO sequent_backend.tenant VALUES
        (council, 'council', look || '{"theme": {"brand": "#000"}}', '{"language": "en"}', true),
        (board, 'board', NULL, '{}', false);
    IF EXISTS (SELECT FROM sequent_backend.tenant_for_sign_in()) THEN
        RAISE EXCEPTION 'test failed: tenants listed without a slug or id';
    END IF;
    IF EXISTS (SELECT FROM sequent_backend.tenant_for_sign_in(slug => 'Council'))
       OR EXISTS (SELECT FROM sequent_backend.tenant_for_sign_in(slug => 'council', id => board)) THEN
        RAISE EXCEPTION 'test failed: a tenant matched another slug or id';
    END IF;
    IF (SELECT array_agg(row(id, annotations)) FROM sequent_backend.tenant_for_sign_in(slug => 'council'))
           IS DISTINCT FROM ARRAY[row(council, look)]
       OR (SELECT array_agg(row(id, annotations)) FROM sequent_backend.tenant_for_sign_in(id => board))
           IS DISTINCT FROM ARRAY[row(board, '{}'::jsonb)]
       OR (SELECT count(*) FROM sequent_backend.tenant_for_sign_in(slug => 'council', id => council)) <> 1 THEN
        RAISE EXCEPTION 'test failed: wrong tenant or columns';
    END IF;
    IF (SELECT array_agg(attname::text ORDER BY attnum) FROM pg_attribute
        WHERE attrelid = 'sequent_backend.sign_in_tenant'::regclass AND attnum > 0) <> ARRAY['id', 'annotations']
       OR (SELECT provolatile FROM pg_proc WHERE oid = 'sequent_backend.tenant_for_sign_in'::regproc) <> 's' THEN
        RAISE EXCEPTION 'test failed: the lookup exposes more than id and look';
    END IF;
    RAISE NOTICE 'PASS: no listing, exact slug and id matches, id and logo and style only, stable';
END;
$$;
\ir ../migrations/backend-db/1793020020000_tenant_for_sign_in/down.sql
ROLLBACK;
