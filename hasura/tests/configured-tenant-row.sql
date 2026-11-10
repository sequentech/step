-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-- Run with psql -v ON_ERROR_STOP=1 -f this-file.sql against a disposable empty database.
BEGIN;
CREATE SCHEMA sequent_backend;
CREATE TABLE sequent_backend.tenant (id uuid PRIMARY KEY, settings jsonb);
CREATE TABLE sequent_backend.election_event (tenant_id uuid, is_archived boolean);
\ir ../migrations/backend-db/1793020000000_configuration_signing_settings/up.sql
\ir ../migrations/backend-db/1793020020001_configured_tenant_row/up.sql
DO $$
DECLARE
    configured uuid := '00000000-0000-4000-8000-000000000001';
    plain uuid := '00000000-0000-4000-8000-000000000002';
    other uuid := '00000000-0000-4000-8000-000000000003';
    -- Structural PEM fixture; full X.509 validation belongs to import and UI tests.
    root text := E'-----BEGIN CERTIFICATE-----\nY2E=\n-----END CERTIFICATE-----\n';
    cfg jsonb;
    changed jsonb;
BEGIN
    cfg := jsonb_build_object('policy', 'required', 'package_roots', root, 'staff_roots', root, 'required_approvals', 2);
    changed := jsonb_build_object('configuration_signing', cfg || '{"required_approvals": 3}');
    INSERT INTO sequent_backend.tenant VALUES (configured, jsonb_build_object('configuration_signing', cfg)), (plain, '{}');
    BEGIN
        DELETE FROM sequent_backend.tenant WHERE id = configured;
        RAISE EXCEPTION 'test failed: configured tenant deleted';
    EXCEPTION WHEN insufficient_privilege THEN NULL;
    END;
    BEGIN
        UPDATE sequent_backend.tenant SET id = other WHERE id = configured;
        RAISE EXCEPTION 'test failed: configured tenant moved to another id';
    EXCEPTION WHEN insufficient_privilege THEN NULL;
    END;
    BEGIN
        INSERT INTO sequent_backend.tenant VALUES (configured, changed);
        RAISE EXCEPTION 'test failed: configured tenant inserted twice';
    EXCEPTION WHEN unique_violation THEN NULL;
    END;
    BEGIN
        INSERT INTO sequent_backend.tenant VALUES (configured, changed)
            ON CONFLICT (id) DO UPDATE SET settings = EXCLUDED.settings;
        RAISE EXCEPTION 'test failed: trust replaced by an upsert';
    EXCEPTION WHEN insufficient_privilege THEN NULL;
    END;
    -- A full-record update writes the unchanged id.
    UPDATE sequent_backend.tenant SET id = configured, settings = settings || '{"language": "de"}' WHERE id = configured;
    IF (SELECT settings -> 'configuration_signing' FROM sequent_backend.tenant WHERE id = configured) IS DISTINCT FROM cfg THEN
        RAISE EXCEPTION 'test failed: configured trust changed';
    END IF;
    UPDATE sequent_backend.tenant SET id = other WHERE id = plain;
    DELETE FROM sequent_backend.tenant WHERE id = other;
    INSERT INTO sequent_backend.tenant VALUES (plain, jsonb_build_object('configuration_signing', cfg));
    RAISE NOTICE 'PASS: delete, id change, second insert and upsert refused; unchanged id, unconfigured tenant and initial configuration accepted';
END;
$$;
\ir ../migrations/backend-db/1793020020001_configured_tenant_row/down.sql
DELETE FROM sequent_backend.tenant WHERE id = '00000000-0000-4000-8000-000000000001';
ROLLBACK;
