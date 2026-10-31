-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Log entries of administrative actions, written in the action's transaction
-- and posted in `seq` order per log by the outbox worker. `id` is the entry
-- id the worker looks for on the log before posting. A row without an
-- election event belongs to the tenant log. Rows outlive the election event
-- they name, so nothing here references it.
CREATE TABLE sequent_backend.audit_log_outbox (
    id uuid NOT NULL DEFAULT gen_random_uuid(),
    seq bigserial NOT NULL,
    tenant_id uuid NOT NULL,
    election_event_id uuid,
    statement_kind text NOT NULL,
    event_type text NOT NULL,
    log_type text NOT NULL,
    user_id text,
    username text,
    election_id uuid,
    area_id uuid,
    description text NOT NULL,
    details jsonb NOT NULL,
    occurred_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    posted_at timestamptz,
    attempts integer NOT NULL DEFAULT 0,
    last_error text,
    last_attempt_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (id),
    CONSTRAINT audit_log_outbox_seq_key UNIQUE (seq),
    -- electoral_log's audit statement kinds: an unknown kind would stop the
    -- worker at that row.
    CONSTRAINT audit_log_outbox_statement_kind_known CHECK (statement_kind IN (
        'AdminChange', 'KeycloakAdminEvent', 'LogsExported', 'LogWriterStarted',
        'LogWriterStopped'
    )),
    CONSTRAINT audit_log_outbox_event_type_known CHECK (event_type IN ('USER', 'SYSTEM')),
    CONSTRAINT audit_log_outbox_log_type_known CHECK (log_type IN ('INFO', 'ERROR')),
    CONSTRAINT audit_log_outbox_attempts_count CHECK (attempts >= 0)
);

-- What the worker still has to post.
CREATE INDEX audit_log_outbox_unposted
    ON sequent_backend.audit_log_outbox (tenant_id, election_event_id, seq)
    WHERE posted_at IS NULL;

-- The redaction list. A path names a secret when one of its dot-separated
-- segments, read as snake_case words whatever its case, ends in a secret's
-- name: password, secret, token, credential(s), authorization, or a key
-- qualified as api, gapi, private, secret, signing, access, auth, client or
-- encryption (`smtp_password`, `clientSecret`, `AWS_SECRET_ACCESS_KEY`),
-- optionally followed by hash, salt or, for an access key, id. Names that only
-- start with or contain such a word (`password_policy`, `tokenUrl`,
-- `recaptcha_site_key`, `public_key`) are not secrets.
CREATE FUNCTION sequent_backend.audit_is_secret(path text)
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

-- A value logged whole (an array, or a value that replaces one of another
-- type) with the secrets inside it blanked.
CREATE FUNCTION sequent_backend.audit_redact(value jsonb)
RETURNS jsonb
LANGUAGE plpgsql
IMMUTABLE
AS $$
BEGIN
    CASE jsonb_typeof(value)
        WHEN 'object' THEN
            RETURN COALESCE((
                SELECT jsonb_object_agg(
                    member.key,
                    CASE
                        WHEN sequent_backend.audit_is_secret(member.key)
                            THEN '{"secret": true}'::jsonb
                        ELSE sequent_backend.audit_redact(member.value)
                    END
                )
                FROM jsonb_each(value) AS member
            ), '{}'::jsonb);
        WHEN 'array' THEN
            RETURN COALESCE((
                SELECT jsonb_agg(sequent_backend.audit_redact(item.value) ORDER BY item.position)
                FROM jsonb_array_elements(value) WITH ORDINALITY AS item(value, position)
            ), '[]'::jsonb);
        ELSE
            RETURN value;
    END CASE;
END;
$$;

-- The changes between two values of `path` as AdminChange `changes`. A SQL
-- NULL is a value that is not there. Objects are compared key by key under
-- dotted paths; anything else is compared whole.
CREATE FUNCTION sequent_backend.audit_changes(path text, old_value jsonb, new_value jsonb)
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

-- The name an entry shows for a row: its naming column or, for the records
-- that keep their name in `presentation`, the name in the default language,
-- in English or in the first language that has one.
CREATE FUNCTION sequent_backend.audit_object_name(entity jsonb)
RETURNS text
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT COALESCE(
        NULLIF(entity ->> 'name', ''),
        NULLIF(entity ->> 'alias', ''),
        NULLIF(entity ->> 'slug', ''),
        NULLIF(entity ->> 'title', ''),
        NULLIF(entity ->> 'username', ''),
        NULLIF(entity ->> 'email', ''),
        NULLIF(entity #>> ARRAY[
            'presentation', 'i18n',
            entity #>> '{presentation,language_conf,default_language_code}', 'name'
        ], ''),
        NULLIF(entity #>> '{presentation,i18n,en,name}', ''),
        (
            SELECT language.value ->> 'name'
            FROM jsonb_each(
                CASE jsonb_typeof(entity #> '{presentation,i18n}')
                    WHEN 'object' THEN entity #> '{presentation,i18n}'
                    ELSE '{}'::jsonb
                END
            ) AS language
            WHERE jsonb_typeof(language.value) = 'object'
              AND NULLIF(language.value ->> 'name', '') IS NOT NULL
            ORDER BY language.key COLLATE "C"
            LIMIT 1
        )
    );
$$;

-- Whether a deleted row went with a parent it references ON DELETE CASCADE.
-- The cascade's triggers fire at the depth of the statement that deleted the
-- parent, so pg_trigger_depth() cannot tell them apart; the parent being gone
-- can.
CREATE FUNCTION sequent_backend.audit_deleted_by_cascade(relation oid, deleted jsonb)
RETURNS boolean
LANGUAGE plpgsql
AS $$
DECLARE
    parent_is_gone text;
    cascaded boolean;
BEGIN
    FOR parent_is_gone IN
        SELECT format(
            'SELECT NOT EXISTS (SELECT 1 FROM %s AS parent WHERE %s)',
            reference.confrelid::regclass,
            string_agg(
                format(
                    'parent.%I = ($1 ->> %L)::%s',
                    parent_column.attname,
                    child_column.attname,
                    format_type(parent_column.atttypid, parent_column.atttypmod)
                ),
                ' AND '
            )
        )
        FROM pg_constraint AS reference
        CROSS JOIN LATERAL unnest(reference.conkey, reference.confkey) AS key(child, parent)
        JOIN pg_attribute AS child_column
            ON child_column.attrelid = reference.conrelid AND child_column.attnum = key.child
        JOIN pg_attribute AS parent_column
            ON parent_column.attrelid = reference.confrelid AND parent_column.attnum = key.parent
        WHERE reference.conrelid = relation
          AND reference.contype = 'f'
          AND reference.confdeltype = 'c'
        GROUP BY reference.oid, reference.confrelid
        HAVING bool_and(deleted ->> child_column.attname IS NOT NULL)
    LOOP
        EXECUTE parent_is_gone INTO cascaded USING deleted;
        IF cascaded THEN
            RETURN true;
        END IF;
    END LOOP;
    RETURN false;
END;
$$;

-- Writes the AdminChange entry of a row changed through Hasura by a person.
-- Statements without a Hasura session, the admin secret and the service
-- account are Step's own writes, which log themselves; rows written by
-- another trigger or deleted by a cascade are not the person's own statement.
--
-- A row of an election event goes to that event's log and any other row to
-- the tenant log. Creating, deleting, archiving and unarchiving an election
-- event are tenant-log entries; its other changes go to its own log.
CREATE FUNCTION sequent_backend.audit_change()
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

-- Every table a Hasura role other than service-account can insert into,
-- update or delete from. hasura/tests/audit-change-coverage.test.cjs compares
-- this list with the metadata.
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.applications
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.area
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.area_contest
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.ballot_publication
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.candidate
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.certificate_authority
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.contest
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.document
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.election
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.election_event
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.election_type
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.event_execution
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.notification
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.phone_blacklist
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.preview
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.report
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.scheduled_event
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.support_material
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.tally_session
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.tally_session_contest
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.tally_session_execution
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.tally_session_resolution
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.tally_sheet
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.template
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.tenant
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
CREATE TRIGGER audit_change AFTER INSERT OR UPDATE OR DELETE ON sequent_backend.trustee
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.audit_change();
