-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- SW-F-0364: what Harvest and Windmill change for an admin is logged like
-- what a person changes through Hasura. The code names the admin with the
-- transaction setting `sequent.audit_actor` (windmill's audit_log::act_for):
-- {user_id, username, role, origin, tenant_id}. A Hasura session still takes
-- precedence, and a statement with neither writes nothing, as before.

-- A key qualified as a webhook's authenticates the provider's callbacks
-- (messaging_account.webhook_key): a secret, as in windmill's is_secret_path.
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
                    || '|(api|gapi|private|secret|signing|access|auth|client|encryption|webhook)_keys?)'
                    || '(_(hash|salt))?$')
    );
$$;


-- Nested keys set to JSON null are no change from keys left out: saving a form
-- that writes every key of an object logs only what it changed.
CREATE OR REPLACE FUNCTION sequent_backend.audit_changes(path text, old_value jsonb, new_value jsonb)
RETURNS jsonb
LANGUAGE plpgsql
IMMUTABLE
AS $$
DECLARE
    -- Longer texts are logged as their SHA-256.
    max_value_bytes CONSTANT integer := 65536;
    changes jsonb := '[]'::jsonb;
    change jsonb := jsonb_build_object('path', path);
    member text;
    old_text text;
    new_text text;
BEGIN
    -- A key set to JSON null is a key left out, as in windmill's diff_values.
    old_value := NULLIF(old_value, 'null'::jsonb);
    new_value := NULLIF(new_value, 'null'::jsonb);
    IF old_value IS NOT DISTINCT FROM new_value THEN
        RETURN changes;
    END IF;
    IF sequent_backend.audit_is_secret(path) THEN
        RETURN jsonb_build_array(change || '{"secret": true}'::jsonb);
    END IF;

    IF (old_value IS NULL OR jsonb_typeof(old_value) = 'object')
       AND (new_value IS NULL OR jsonb_typeof(new_value) = 'object')
       AND COALESCE(old_value, '{}'::jsonb) || COALESCE(new_value, '{}'::jsonb) <> '{}'::jsonb THEN
        FOR member IN
            SELECT key
            FROM jsonb_object_keys(COALESCE(old_value, '{}'::jsonb) || COALESCE(new_value, '{}'::jsonb)) AS key
            ORDER BY key COLLATE "C"
        LOOP
            changes := changes || sequent_backend.audit_changes(
                path || '.' || member, old_value -> member, new_value -> member
            );
        END LOOP;
        RETURN changes;
    END IF;

    old_value := sequent_backend.audit_redact(old_value);
    new_value := sequent_backend.audit_redact(new_value);
    old_text := CASE jsonb_typeof(old_value)
        WHEN 'string' THEN old_value #>> '{}' ELSE old_value::text END;
    new_text := CASE jsonb_typeof(new_value)
        WHEN 'string' THEN new_value #>> '{}' ELSE new_value::text END;
    IF GREATEST(octet_length(old_text), octet_length(new_text)) > max_value_bytes THEN
        IF old_value IS NOT NULL THEN
            change := change || jsonb_build_object(
                'old_sha256', encode(sha256(convert_to(old_text, 'UTF8')), 'hex')
            );
        END IF;
        IF new_value IS NOT NULL THEN
            change := change || jsonb_build_object(
                'new_sha256', encode(sha256(convert_to(new_text, 'UTF8')), 'hex')
            );
        END IF;
    ELSE
        IF old_value IS NOT NULL THEN
            change := change || jsonb_build_object('old', old_value);
        END IF;
        IF new_value IS NOT NULL THEN
            change := change || jsonb_build_object('new', new_value);
        END IF;
    END IF;
    RETURN jsonb_build_array(change);
END;
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
    actor jsonb;
    origin text;
    session_tenant text;
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
    IF pg_trigger_depth() > 1 THEN
        RETURN NULL;
    END IF;
    hasura_session := current_setting('hasura.user', true);
    IF hasura_session IS NOT NULL AND hasura_session <> '' THEN
        session := hasura_session::jsonb;
        IF session ->> 'x-hasura-role' IN ('admin', 'service-account') THEN
            RETURN NULL;
        END IF;
        actor := jsonb_build_object(
            'user_id', session ->> 'x-hasura-user-id',
            'username', NULL,
            'role', session ->> 'x-hasura-role'
        );
        origin := 'hasura';
        session_tenant := session ->> 'x-hasura-tenant-id';
    ELSE
        session := NULLIF(current_setting('sequent.audit_actor', true), '')::jsonb;
        IF session IS NULL THEN
            RETURN NULL;
        END IF;
        actor := jsonb_build_object(
            'user_id', session ->> 'user_id',
            'username', session ->> 'username',
            'role', session ->> 'role'
        );
        origin := session ->> 'origin';
        session_tenant := session ->> 'tenant_id';
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
        tenant_id, election_event_id, statement_kind, event_type, log_type, user_id, username,
        election_id, area_id, description, details
    )
    SELECT
        COALESCE(
            entity ->> CASE TG_TABLE_NAME WHEN 'tenant' THEN 'id' ELSE 'tenant_id' END,
            session_tenant
        )::uuid,
        entry.election_event_id,
        'AdminChange',
        CASE WHEN actor ->> 'user_id' IS NULL THEN 'SYSTEM' ELSE 'USER' END,
        'INFO',
        actor ->> 'user_id',
        actor ->> 'username',
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
            'actor', actor,
            'origin', origin,
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

-- Tables only Step's own code writes, for an admin. What they hold is
-- configuration or a decision: ceremonies, messaging accounts and settings,
-- results publications, tally sheet imports, signing settings and staff
-- certificates, monitoring settings.
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.keys_ceremony
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.messaging_account
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.messaging_event_config
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.monitoring_config
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.signing_checks
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.staff_certificate
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.tally_results_publication
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.tally_sheet_import
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.tally_sheet_import_item
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
