-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
\set ON_ERROR_STOP on
CREATE SCHEMA sequent_backend;
CREATE TABLE sequent_backend.election_event(id uuid PRIMARY KEY,tenant_id uuid,presentation jsonb);
CREATE TABLE sequent_backend.election(id uuid PRIMARY KEY,tenant_id uuid,election_event_id uuid,presentation jsonb);
CREATE TABLE sequent_backend.election_lifecycle(tenant_id uuid,election_event_id uuid,phase text);
\ir ../migrations/backend-db/1793020006000_administrator_settings/up.sql
INSERT INTO sequent_backend.election_event VALUES('33333333-3333-4333-8333-333333333333','11111111-1111-4111-8111-111111111111','{"i18n":{"en":{"name":"Council election","description":"private election data"}},"otp":{"policy":"required"},"dates":{"start":"2030-01-01"},"materials":{"ballot":"secret"}}');
INSERT INTO sequent_backend.election_lifecycle VALUES('11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333','preparation');
DO $$ DECLARE projected jsonb; BEGIN
    SELECT administration_settings INTO projected FROM sequent_backend.election_event;
    IF projected <> '{"i18n":{"en":{"name":"Council election"}},"otp":{"policy":"required"}}'::jsonb THEN RAISE EXCEPTION 'Administrator projection exposed election data: %',projected; END IF;
    IF sequent_backend.administration_settings('null') <> '{"i18n":{}}'::jsonb THEN RAISE EXCEPTION 'Null presentation failed'; END IF;
END $$;
SET "hasura.user"='{"x-hasura-role":"admin-user"}';
UPDATE sequent_backend.election_event SET presentation='{"otp":{"policy":"stronger"},"i18n":{"en":{"name":"Council election"}}}';
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM sequent_backend.election_event WHERE presentation->'materials'='{"ballot":"secret"}'::jsonb AND presentation->'otp'='{"policy":"stronger"}'::jsonb) THEN RAISE EXCEPTION 'Settings save discarded protected election data'; END IF;
    BEGIN
        UPDATE sequent_backend.election_event SET presentation='{"dates":{"start":"2031-01-01"}}';
        RAISE EXCEPTION 'Administrator changed election dates';
    EXCEPTION WHEN insufficient_privilege THEN NULL; END;
    BEGIN
        UPDATE sequent_backend.election_event SET presentation='{"i18n":{"en":{"name":"changed ballot display"}}}';
        RAISE EXCEPTION 'Administrator changed ballot display';
    EXCEPTION WHEN insufficient_privilege THEN NULL; END;
END $$;
UPDATE sequent_backend.election_lifecycle SET phase='execution';
DO $$ BEGIN
    BEGIN
        UPDATE sequent_backend.election_event SET presentation='{"otp":{"policy":"disabled"}}';
        RAISE EXCEPTION 'Administrator changed settings after Preparation';
    EXCEPTION WHEN insufficient_privilege THEN NULL; END;
END $$;
SELECT 'PASS: administrator sees security settings only, preserves hidden ballot data on save, cannot write dates/display data or alter settings outside Preparation' AS result;
