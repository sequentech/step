-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-- A fresh disposable database is mandatory: schema creation refuses an existing one.
\set ON_ERROR_STOP on
CREATE SCHEMA sequent_backend;
\ir ../migrations/backend-db/1793020004000_security_notifications/up.sql
DO $$ BEGIN
    IF sequent_backend.audit_storage_allows_voters() THEN RAISE EXCEPTION 'Missing storage state admitted voters'; END IF;
END $$;
INSERT INTO sequent_backend.audit_storage_status(singleton,state) VALUES(true,'available');
DO $$ BEGIN
    IF NOT sequent_backend.audit_storage_allows_voters() THEN RAISE EXCEPTION 'Fresh healthy observation refused'; END IF;
END $$;
UPDATE sequent_backend.audit_storage_status SET updated_at=clock_timestamp()-interval '46 seconds';
DO $$ BEGIN
    IF sequent_backend.audit_storage_allows_voters() THEN RAISE EXCEPTION 'Expired observation admitted voters'; END IF;
END $$;
UPDATE sequent_backend.audit_storage_status SET updated_at=clock_timestamp()+interval '1 minute';
DO $$ BEGIN
    IF sequent_backend.audit_storage_allows_voters() THEN RAISE EXCEPTION 'Future observation admitted voters'; END IF;
END $$;
UPDATE sequent_backend.audit_storage_status SET updated_at=clock_timestamp(),state='threshold-exceeded';
DO $$ BEGIN
    IF sequent_backend.audit_storage_allows_voters() THEN RAISE EXCEPTION 'Audit threshold admitted voters'; END IF;
END $$;
UPDATE sequent_backend.audit_storage_status SET state='unavailable';
DO $$ BEGIN
    IF sequent_backend.audit_storage_allows_voters() THEN RAISE EXCEPTION 'Unavailable storage admitted voters'; END IF;
END $$;
UPDATE sequent_backend.audit_storage_status SET state='disabled';
DO $$ BEGIN
    IF NOT sequent_backend.audit_storage_allows_voters() THEN RAISE EXCEPTION 'Explicit non-evaluated policy was ignored'; END IF;
END $$;
UPDATE sequent_backend.audit_storage_status SET updated_at=clock_timestamp()-interval '46 seconds';
DO $$ BEGIN
    IF sequent_backend.audit_storage_allows_voters() THEN RAISE EXCEPTION 'Expired policy heartbeat admitted voters'; END IF;
END $$;
BEGIN;
INSERT INTO sequent_backend.security_notification(id,tenant_id,election_event_id,kind,details)
VALUES('11111111-1111-4111-8111-111111111111','22222222-2222-4222-8222-222222222222','33333333-3333-4333-8333-333333333333','fixture','rollback');
ROLLBACK;
DO $$ BEGIN
    IF EXISTS(SELECT 1 FROM sequent_backend.security_notification) THEN RAISE EXCEPTION 'Rolled-back security event left an email'; END IF;
END $$;
SELECT 'PASS: measured storage absence, threshold, expiry, future timestamps, deployment policy and transactional email outbox' AS result;
