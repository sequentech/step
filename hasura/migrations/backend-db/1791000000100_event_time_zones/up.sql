-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- VOTE-LIFECYCLE: an election event's timezones live in its presentation
-- (`presentation.timezones`), and only there. The monitoring settings named
-- the event's zone before (`time_zone`); they no longer may.
--
-- 1. Every event without `presentation.timezones` gets one zone, Z, as its
--    configured and primary zone: the zone its live monitoring settings
--    name, else UTC. Logs show each row's election zone (the default).
-- 2. Each live monitoring settings document that names a zone gets a new
--    revision without the `time_zone` line, as a change of its own: the
--    event's configuration generation moves on by one and the head moves to
--    the new revision. The revisions before it stay as they were saved (they
--    are append-only, and the electoral log holds their digests). The new
--    revision keeps the origin and preset of the one it follows (settings
--    are only written by a reset to a preset) and is by the author
--    `system:1791000000100_event_time_zones`, which down.sql looks for.
--
-- The zone is read as the settings' top-level one-line `time_zone: <zone>`
-- (quoted or not, spaces before the colon and a trailing comment allowed),
-- the way the presets wrote it. Anything else is left as it is and named in
-- a WARNING, as is a value that isn't a zone name (the event gets UTC) and a
-- presentation that isn't a JSON object (it becomes one holding only the
-- timezones). Stored instants don't change.
--
-- Release note: the publication digest covers the event presentation, so a
-- waiting Approve configuration request no longer matches once this has
-- run. Finish or cancel those requests before migrating.

DO $$
DECLARE
    source record;
    generation bigint;
    zone text;
    stripped text;
    left_over bigint;
BEGIN
    FOR source IN
        SELECT tenant_id, id, jsonb_typeof(presentation) AS kind
        FROM sequent_backend.election_event
        WHERE presentation IS NOT NULL AND jsonb_typeof(presentation) <> 'object'
          AND jsonb_typeof(presentation) <> 'null'
        ORDER BY tenant_id, id
    LOOP
        RAISE WARNING 'Election event % (tenant %): its presentation is a JSON %, not an object; it becomes an object holding only its timezones',
            source.id, source.tenant_id, source.kind;
    END LOOP;

    FOR source IN
        SELECT c.*
        FROM sequent_backend.monitoring_config_head h
        JOIN sequent_backend.monitoring_config c
          ON c.tenant_id = h.tenant_id AND c.election_event_id = h.election_event_id
         AND c.kind = h.kind AND c.key = h.key AND c.revision = h.revision
        WHERE h.kind = 'settings' AND c.change = 'UPSERT'
          AND c.yaml ~ '(?n)^time_zone[ \t]*:'
        ORDER BY c.tenant_id, c.election_event_id, c.key
    LOOP
        zone := substring(source.yaml FROM
            '(?n)^time_zone[ \t]*:[ \t]*[''"]?([A-Za-z0-9_+/-]+)[''"]?[ \t]*(?:#[^\n]*)?$');
        IF zone IS NULL THEN
            RAISE WARNING 'Election event % (tenant %): its monitoring settings name time_zone in a form this migration does not rewrite; they are left as they are (they no longer parse: remove the line by hand), and the event gets UTC',
                source.election_event_id, source.tenant_id;
            CONTINUE;
        END IF;
        IF zone !~ '^(UTC|[A-Za-z][A-Za-z0-9_+-]*(/[A-Za-z0-9_+-]+)+)$' THEN
            RAISE WARNING 'Election event % (tenant %): its monitoring time_zone % is not a zone name; the event gets UTC',
                source.election_event_id, source.tenant_id, zone;
            zone := 'UTC';
        END IF;

        UPDATE sequent_backend.election_event e
        SET presentation = jsonb_set(
            CASE WHEN jsonb_typeof(e.presentation) = 'object' THEN e.presentation
                 ELSE '{}'::jsonb END,
            '{timezones}',
            jsonb_build_object(
                'configured', jsonb_build_array(zone), 'primary', zone, 'logs', 'election'
            )
        )
        WHERE e.tenant_id = source.tenant_id AND e.id = source.election_event_id
          AND COALESCE(jsonb_typeof(e.presentation->'timezones'), 'null') = 'null';

        stripped := regexp_replace(source.yaml, '^time_zone[ \t]*:[^\n]*(\n|$)', '', 'n');
        UPDATE sequent_backend.monitoring_event
        SET config_generation = config_generation + 1, updated_at = now()
        WHERE tenant_id = source.tenant_id AND election_event_id = source.election_event_id
        RETURNING config_generation INTO generation;
        UPDATE sequent_backend.monitoring_config_head
        SET revision = revision + 1, updated_at = now()
        WHERE tenant_id = source.tenant_id AND election_event_id = source.election_event_id
          AND kind = source.kind AND key = source.key AND revision = source.revision;
        INSERT INTO sequent_backend.monitoring_config
            (tenant_id, election_event_id, kind, key, revision, change, yaml, sha256,
             origin, preset_id, preset_version, config_generation, author_id, author_name)
        VALUES
            (source.tenant_id, source.election_event_id, source.kind, source.key,
             source.revision + 1, 'UPSERT', stripped,
             encode(sha256(convert_to(stripped, 'UTF8')), 'hex'),
             source.origin, source.preset_id, source.preset_version, generation,
             'system:1791000000100_event_time_zones',
             'Time zone moved to the event presentation');
    END LOOP;

    -- Every other event without timezones: UTC.
    UPDATE sequent_backend.election_event e
    SET presentation = jsonb_set(
        CASE WHEN jsonb_typeof(e.presentation) = 'object' THEN e.presentation
             ELSE '{}'::jsonb END,
        '{timezones}',
        jsonb_build_object(
            'configured', jsonb_build_array('UTC'), 'primary', 'UTC', 'logs', 'election'
        )
    )
    WHERE COALESCE(jsonb_typeof(e.presentation->'timezones'), 'null') = 'null';

    SELECT count(*) INTO left_over
    FROM sequent_backend.monitoring_config_head h
    JOIN sequent_backend.monitoring_config c
      ON c.tenant_id = h.tenant_id AND c.election_event_id = h.election_event_id
     AND c.kind = h.kind AND c.key = h.key AND c.revision = h.revision
    WHERE h.kind = 'settings' AND c.yaml ~ '(^|[[:space:]{,])time_zone[[:space:]]*:';
    IF left_over > 0 THEN
        RAISE WARNING '% monitoring settings still name a time_zone; edit them by hand', left_over;
    END IF;
END;
$$;
