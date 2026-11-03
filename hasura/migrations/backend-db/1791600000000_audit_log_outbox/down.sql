-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP TRIGGER IF EXISTS audit_change ON sequent_backend.applications;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.area;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.area_contest;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.ballot_publication;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.candidate;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.certificate_authority;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.contest;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.document;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.election;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.election_event;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.election_type;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.event_execution;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.notification;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.phone_blacklist;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.preview;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.report;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.scheduled_event;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.support_material;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.tally_session;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.tally_session_contest;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.tally_session_execution;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.tally_session_resolution;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.tally_sheet;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.template;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.tenant;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.trustee;

DROP FUNCTION IF EXISTS sequent_backend.audit_change();
DROP FUNCTION IF EXISTS sequent_backend.audit_deleted_by_cascade(oid, jsonb);
DROP FUNCTION IF EXISTS sequent_backend.audit_object_name(jsonb);
DROP FUNCTION IF EXISTS sequent_backend.audit_changes(text, jsonb, jsonb);
DROP FUNCTION IF EXISTS sequent_backend.audit_redact(jsonb);
DROP FUNCTION IF EXISTS sequent_backend.audit_is_secret(text);

DROP TABLE IF EXISTS sequent_backend.audit_log_outbox;
