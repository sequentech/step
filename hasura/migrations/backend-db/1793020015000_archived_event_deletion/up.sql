-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- An archived event never opens voting and stays in preparation: the board exports
-- and deletes its data there. Every other event still needs post-processing.
CREATE OR REPLACE FUNCTION sequent_backend.authorized_execution_deletion(tenant uuid,event uuid)
RETURNS boolean LANGUAGE sql STABLE AS $$
SELECT sequent_backend.trusted_write_allowed() AND EXISTS (
    SELECT 1 FROM sequent_backend.board_operation_binding b
    JOIN sequent_backend.signing_request r ON r.id=b.request_id
    JOIN sequent_backend.election_lifecycle l ON l.election_event_id=b.election_event_id AND l.tenant_id=b.tenant_id
    JOIN sequent_backend.election_event e ON e.id=b.election_event_id AND e.tenant_id=b.tenant_id
    JOIN sequent_backend.election_execution_revision v ON v.election_event_id=b.election_event_id AND v.tenant_id=b.tenant_id
    JOIN sequent_backend.execution_export x ON x.election_event_id=b.election_event_id AND x.tenant_id=b.tenant_id
    WHERE b.tenant_id=tenant AND b.election_event_id=event AND b.binding_kind='deletion'
      AND b.request_id::text=current_setting('sequent.authorized_deletion',true)
      AND r.action='delete-execution-data' AND r.status='executed'
      AND (l.phase='post_processing' OR (l.phase='preparation' AND e.is_archived IS TRUE))
      AND l.generation=x.lifecycle_generation
      AND v.revision=x.source_revision
      AND r.subject->>'lifecycle_generation'=l.generation::text
      AND r.subject->>'source_revision'=v.revision::text
);
$$;

-- Nothing moves the lifecycle of an archived event out of preparation.
CREATE FUNCTION sequent_backend.keep_archived_event_in_preparation()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF OLD.phase='preparation' AND NEW.phase<>'preparation' AND EXISTS (
        SELECT 1 FROM sequent_backend.election_event e
        WHERE e.id=NEW.election_event_id AND e.tenant_id=NEW.tenant_id AND e.is_archived IS TRUE) THEN
        RAISE EXCEPTION 'An archived election event stays in preparation' USING ERRCODE='42501';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER keep_archived_event_in_preparation BEFORE UPDATE OF phase
ON sequent_backend.election_lifecycle FOR EACH ROW
EXECUTE FUNCTION sequent_backend.keep_archived_event_in_preparation();

-- A document's name is part of its storage key and of the paths it is
-- uploaded and downloaded through. A generated name may separate the parts
-- of its path with '/'.
CREATE FUNCTION sequent_backend.guard_document_name()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.name ~ '[[:cntrl:]\\;#%?]' AND (TG_OP='INSERT' OR NEW.name IS DISTINCT FROM OLD.name) THEN
        RAISE EXCEPTION USING ERRCODE='23514', CONSTRAINT='document_name_valid',
            MESSAGE='A document name cannot contain \ ; # % ? or a control character. Rename it and try again.';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER guard_document_name BEFORE INSERT OR UPDATE OF name
ON sequent_backend.document FOR EACH ROW
EXECUTE FUNCTION sequent_backend.guard_document_name();
