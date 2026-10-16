-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Restores audit_is_secret() and audit_change() as
-- 1791600000000_audit_log_outbox wrote them.
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.keys_ceremony;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.messaging_account;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.messaging_event_config;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.monitoring_config;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.signing_checks;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.staff_certificate;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.tally_results_publication;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.tally_sheet_import;
DROP TRIGGER IF EXISTS audit_change ON sequent_backend.tally_sheet_import_item;

CREATE OR REPLACE FUNCTION sequent_backend.audit_is_secret(path text)
RETURNS boolean
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT EXISTS (
        SELECT 1
        FROM unnest(string_to_array(path, '.')) AS segment
        WHERE lower(regexp_replace(
                  regexp_replace(segment, '([a-z0-9])([A-Z])', '\1_\2', 'g'),
                  '[^A-Za-z0-9]+', '_', 'g'
              )) ~ ('(^|_)(passwords?|passwd|pwd|passphrase|secrets?|tokens?|credentials?'
                    || '|authorization|apikeys?|access_key_id'
                    || '|(api|gapi|private|secret|signing|access|auth|client|encryption)_keys?)'
                    || '(_(hash|salt))?$')
    );
$$;

CREATE OR REPLACE FUNCTION sequent_backend.audit_change()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    bookkeeping CONSTANT text[] := ARRAY['created_at', 'updated_at', 'last_updated_at'];
    listed_paths CONSTANT integer := 5;
    hasura_session text;
    session jsonb;
    old_record jsonb := '{}'::jsonb;
    new_record jsonb := '{}'::jsonb;
    entity jsonb;
    column_name text;
    column_changes jsonb;
    changes jsonb := '[]'::jsonb;
    archive_changes jsonb := '[]'::jsonb;
    action text;
    archive_action text;
    log_event_id uuid;
    subject text;
BEGIN
    hasura_session := current_setting('hasura.user', true);
    IF pg_trigger_depth() > 1 OR hasura_session IS NULL OR hasura_session = '' THEN
        RETURN NULL;
    END IF;
    session := hasura_session::jsonb;
    IF session ->> 'x-hasura-role' IN ('admin', 'service-account') THEN
        RETURN NULL;
    END IF;

    IF TG_OP <> 'INSERT' THEN
        old_record := to_jsonb(OLD) - bookkeeping;
    END IF;
    IF TG_OP = 'DELETE' AND sequent_backend.audit_deleted_by_cascade(TG_RELID, old_record) THEN
        RETURN NULL;
    END IF;
    IF TG_OP <> 'DELETE' THEN
        new_record := to_jsonb(NEW) - bookkeeping;
    END IF;
    entity := CASE TG_OP WHEN 'DELETE' THEN old_record ELSE new_record END;
    action := CASE TG_OP WHEN 'INSERT' THEN 'create' WHEN 'DELETE' THEN 'delete' ELSE 'update' END;

    FOR column_name IN
        SELECT key FROM jsonb_object_keys(entity) AS key ORDER BY key COLLATE "C"
    LOOP
        column_changes := sequent_backend.audit_changes(
            column_name,
            NULLIF(old_record -> column_name, 'null'::jsonb),
            NULLIF(new_record -> column_name, 'null'::jsonb)
        );
        IF TG_TABLE_NAME = 'election_event' AND TG_OP = 'UPDATE' AND column_name = 'is_archived' THEN
            archive_changes := column_changes;
        ELSE
            changes := changes || column_changes;
        END IF;
    END LOOP;

    IF TG_TABLE_NAME = 'election_event' THEN
        IF TG_OP = 'UPDATE' THEN
            log_event_id := (entity ->> 'id')::uuid;
            archive_action := CASE WHEN (entity ->> 'is_archived')::boolean
                THEN 'archive' ELSE 'unarchive' END;
        END IF;
    ELSE
        log_event_id := (entity ->> 'election_event_id')::uuid;
    END IF;

    subject := replace(TG_TABLE_NAME, '_', ' ') || COALESCE(
        ' "' || sequent_backend.audit_object_name(entity) || '"',
        ' ' || (entity ->> 'id'),
        ''
    );

    INSERT INTO sequent_backend.audit_log_outbox (
        tenant_id, election_event_id, statement_kind, event_type, log_type, user_id,
        election_id, area_id, description, details
    )
    SELECT
        COALESCE(
            entity ->> CASE TG_TABLE_NAME WHEN 'tenant' THEN 'id' ELSE 'tenant_id' END,
            session ->> 'x-hasura-tenant-id'
        )::uuid,
        entry.election_event_id,
        'AdminChange',
        'USER',
        'INFO',
        session ->> 'x-hasura-user-id',
        (entity ->> CASE TG_TABLE_NAME WHEN 'election' THEN 'id' ELSE 'election_id' END)::uuid,
        (entity ->> CASE TG_TABLE_NAME WHEN 'area' THEN 'id' ELSE 'area_id' END)::uuid,
        CASE entry.action
            WHEN 'create' THEN 'Created ' || subject
            WHEN 'delete' THEN 'Deleted ' || subject
            WHEN 'archive' THEN 'Archived ' || subject
            WHEN 'unarchive' THEN 'Unarchived ' || subject
            ELSE 'Updated ' || subject || ': ' || (
                SELECT string_agg(change.value ->> 'path', ', ' ORDER BY change.position)
                FROM jsonb_array_elements(entry.changes) WITH ORDINALITY AS change(value, position)
                WHERE change.position <= listed_paths
            ) || CASE
                WHEN jsonb_array_length(entry.changes) > listed_paths
                    THEN ' and ' || jsonb_array_length(entry.changes) - listed_paths || ' more'
                ELSE ''
            END
        END,
        jsonb_build_object(
            'object', jsonb_build_object(
                'type', TG_TABLE_NAME,
                'id', entity ->> 'id',
                'name', sequent_backend.audit_object_name(entity)
            ),
            'action', entry.action,
            'actor', jsonb_build_object(
                'user_id', session ->> 'x-hasura-user-id',
                'username', NULL,
                'role', session ->> 'x-hasura-role'
            ),
            'origin', 'hasura',
            'changes', entry.changes
        )
    FROM (VALUES
        (NULL::uuid, archive_action, archive_changes),
        (log_event_id, action, changes)
    ) AS entry(election_event_id, action, changes)
    WHERE jsonb_array_length(entry.changes) > 0;

    RETURN NULL;
END;
$$;
