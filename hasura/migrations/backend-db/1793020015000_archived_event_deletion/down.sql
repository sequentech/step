-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP TRIGGER guard_document_name ON sequent_backend.document;
DROP FUNCTION sequent_backend.guard_document_name();
DROP TRIGGER keep_archived_event_in_preparation ON sequent_backend.election_lifecycle;
DROP FUNCTION sequent_backend.keep_archived_event_in_preparation();

CREATE OR REPLACE FUNCTION sequent_backend.authorized_execution_deletion(tenant uuid,event uuid)
RETURNS boolean LANGUAGE sql STABLE AS $$
SELECT sequent_backend.trusted_write_allowed() AND EXISTS (
    SELECT 1 FROM sequent_backend.board_operation_binding b
    JOIN sequent_backend.signing_request r ON r.id=b.request_id
    JOIN sequent_backend.election_lifecycle l ON l.election_event_id=b.election_event_id AND l.tenant_id=b.tenant_id
    JOIN sequent_backend.election_execution_revision v ON v.election_event_id=b.election_event_id AND v.tenant_id=b.tenant_id
    JOIN sequent_backend.execution_export x ON x.election_event_id=b.election_event_id AND x.tenant_id=b.tenant_id
    WHERE b.tenant_id=tenant AND b.election_event_id=event AND b.binding_kind='deletion'
      AND b.request_id::text=current_setting('sequent.authorized_deletion',true)
      AND r.action='delete-execution-data' AND r.status='executed'
      AND l.phase='post_processing' AND l.generation=x.lifecycle_generation
      AND v.revision=x.source_revision
      AND r.subject->>'lifecycle_generation'=l.generation::text
      AND r.subject->>'source_revision'=v.revision::text
);
$$;
