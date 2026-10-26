-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-- Run with psql -v ON_ERROR_STOP=1 -f this-file.sql against a disposable empty database.
BEGIN;
CREATE SCHEMA sequent_backend;
CREATE TABLE sequent_backend.tenant (id uuid PRIMARY KEY, settings jsonb);
CREATE TABLE sequent_backend.election_event (tenant_id uuid, is_archived boolean);
\ir ../migrations/backend-db/1793020000000_configuration_signing_settings/up.sql
DO $$
DECLARE
    tenant_value uuid := '00000000-0000-4000-8000-000000000001';
    -- Structural PEM fixture; full X.509 validation belongs to import and UI tests.
    root text := E'-----BEGIN CERTIFICATE-----\nY2E=\n-----END CERTIFICATE-----\n';
    cfg jsonb;
    bad jsonb;
BEGIN
    cfg := jsonb_build_object('policy', 'required', 'package_roots', root, 'staff_roots', root, 'required_approvals', 2);
    INSERT INTO sequent_backend.tenant VALUES (tenant_value, jsonb_build_object('language', 'en'));
    FOREACH bad IN ARRAY ARRAY[
        cfg || '{"required_approvals": 1}', cfg || '{"required_approvals": 2.5}',
        cfg || '{"required_approvals": "2"}', cfg || '{"required_approvals": 65536}',
        cfg || '{"policy": "optional"}', cfg || '{"package_roots": ""}',
        cfg || '{"staff_roots": "-----BEGIN PRIVATE KEY-----"}', cfg - 'required_approvals'
    ] LOOP
        BEGIN
            UPDATE sequent_backend.tenant SET settings = jsonb_build_object('configuration_signing', bad) WHERE id = tenant_value;
            RAISE EXCEPTION 'test failed: invalid trust accepted';
        EXCEPTION WHEN check_violation THEN NULL;
        END;
    END LOOP;
    UPDATE sequent_backend.tenant SET settings = settings || jsonb_build_object('configuration_signing', cfg) WHERE id = tenant_value;
    IF (SELECT settings ->> 'language' FROM sequent_backend.tenant WHERE id = tenant_value) <> 'en' THEN
        RAISE EXCEPTION 'unrelated settings lost';
    END IF;
    UPDATE sequent_backend.tenant SET settings = settings || '{"language": "de"}' WHERE id = tenant_value;
    FOREACH bad IN ARRAY ARRAY[cfg || '{"required_approvals": 3}', cfg || jsonb_build_object('package_roots', root || root)] LOOP
        BEGIN
            UPDATE sequent_backend.tenant SET settings = jsonb_build_object('configuration_signing', bad) WHERE id = tenant_value;
            RAISE EXCEPTION 'test failed: immutable trust changed';
        EXCEPTION WHEN insufficient_privilege THEN NULL;
        END;
    END LOOP;
    BEGIN
        UPDATE sequent_backend.tenant SET settings = '{}' WHERE id = tenant_value;
        RAISE EXCEPTION 'test failed: configured trust removed';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
    INSERT INTO sequent_backend.election_event VALUES (tenant_value, false);
    BEGIN
        UPDATE sequent_backend.tenant SET settings = settings || jsonb_build_object('configuration_signing', cfg || jsonb_build_object('staff_roots', root || root)) WHERE id = tenant_value;
        RAISE EXCEPTION 'test failed: staff trust changed during an election';
    EXCEPTION WHEN insufficient_privilege THEN NULL;
    END;
    UPDATE sequent_backend.election_event SET is_archived = true;
    UPDATE sequent_backend.tenant SET settings = settings || jsonb_build_object('configuration_signing', cfg || jsonb_build_object('staff_roots', root || root)) WHERE id = tenant_value;
    RAISE NOTICE 'PASS: initial configuration, 8 malformed inputs, 2 immutable changes, removal, active-election refusal, archived rotation, unrelated settings';
END;
$$;
ROLLBACK;
