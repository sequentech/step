-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Gives each monitoring settings document that up.sql rewrote, and that
-- nobody changed since, its text from before: a new revision that copies
-- the one up.sql followed, so its digest is that revision's again.
--
-- `presentation.timezones` stays: code from before this migration doesn't
-- read it, and an administrator may have changed it since.

DO $$
DECLARE
    source record;
    generation bigint;
BEGIN
    FOR source IN
        SELECT before.*, c.revision AS head_revision
        FROM sequent_backend.monitoring_config_head h
        JOIN sequent_backend.monitoring_config c
          ON c.tenant_id = h.tenant_id AND c.election_event_id = h.election_event_id
         AND c.kind = h.kind AND c.key = h.key AND c.revision = h.revision
        JOIN sequent_backend.monitoring_config before
          ON before.tenant_id = c.tenant_id AND before.election_event_id = c.election_event_id
         AND before.kind = c.kind AND before.key = c.key AND before.revision = c.revision - 1
        WHERE h.kind = 'settings'
          AND c.author_id = 'system:1791000000100_event_time_zones'
        ORDER BY c.tenant_id, c.election_event_id, c.key
    LOOP
        UPDATE sequent_backend.monitoring_event
        SET config_generation = config_generation + 1, updated_at = now()
        WHERE tenant_id = source.tenant_id AND election_event_id = source.election_event_id
        RETURNING config_generation INTO generation;
        UPDATE sequent_backend.monitoring_config_head
        SET revision = revision + 1, updated_at = now()
        WHERE tenant_id = source.tenant_id AND election_event_id = source.election_event_id
          AND kind = source.kind AND key = source.key AND revision = source.head_revision;
        INSERT INTO sequent_backend.monitoring_config
            (tenant_id, election_event_id, kind, key, revision, change, yaml, sha256,
             origin, preset_id, preset_version, config_generation, author_id, author_name)
        VALUES
            (source.tenant_id, source.election_event_id, source.kind, source.key,
             source.head_revision + 1, source.change, source.yaml, source.sha256,
             source.origin, source.preset_id, source.preset_version, generation,
             'system:1791000000100_event_time_zones:down',
             'Time zone moved back to the monitoring settings');
    END LOOP;
END;
$$;
