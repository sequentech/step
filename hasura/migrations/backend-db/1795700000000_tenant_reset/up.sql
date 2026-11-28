-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Tenant reset (Settings -> Configuration package -> Reset): returns a tenant
-- to what it was before its first configuration package (its baseline), so
-- the package and its roster can be imported again. Windmill deletes the
-- tenant's rows and puts the baseline's back in one transaction after setting sequent.tenant_reset to the tenant's id for that
-- transaction only (set_config(..., true)), as the configuration_signing
-- setter does with its own setting.
--
-- 1. tenant.lifecycle_state: RESETTING from the moment a reset claims the
--    tenant until it has finished. A failed reset stays RESETTING and is
--    retried; it is never rolled back.
-- 2. While a tenant is RESETTING, no row of it can be inserted, by any
--    writer (GraphQL, harvest, windmill, beat), except by the reset's own
--    transactions and a new reset task. Updates and deletes are not refused: they cannot bring
--    back what the reset removes.
-- 3. The guards that refuse deleting a row by design (sealed ballot boxes,
--    append-only monitoring and plugin history, initialization report
--    coverage) let the reset's own transaction delete that tenant's rows,
--    and only that tenant's. Their INSERT and UPDATE checks are unchanged.
--    The deployment's operational signing policy, its required tenants, the
--    operator-provisioned setup installation, demo installation and local
--    initialize profile get no exception: a reset keeps them (or refuses).

ALTER TABLE sequent_backend.tenant
    ADD COLUMN lifecycle_state text NOT NULL DEFAULT 'ACTIVE'
    CONSTRAINT tenant_lifecycle_state_check CHECK (lifecycle_state IN ('ACTIVE', 'RESETTING'));

-- What the tenant was before its first configuration package was applied:
-- its tenant-level rows and its tenant realm's users, groups and texts. A
-- reset returns to it. Written by windmill only; not tracked in Hasura. It
-- holds no credentials; secret rows stay encrypted with the master secret.
CREATE TABLE sequent_backend.tenant_reset_baseline (
    tenant_id uuid PRIMARY KEY REFERENCES sequent_backend.tenant(id) ON DELETE CASCADE,
    captured_at timestamptz NOT NULL DEFAULT now(),
    captured_by_task text,
    tenant_row jsonb NOT NULL,
    rows jsonb NOT NULL,
    realm jsonb NOT NULL
);
REVOKE ALL ON sequent_backend.tenant_reset_baseline FROM PUBLIC;

CREATE FUNCTION sequent_backend.tenant_reset_allowed(tenant uuid)
RETURNS boolean
LANGUAGE sql
STABLE
AS $$
    SELECT coalesce(current_setting('sequent.tenant_reset', true), '') = tenant::text;
$$;

CREATE FUNCTION sequent_backend.tenant_reset_refuse_insert()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    -- A reset task is let in: running a reset again is how one that
    -- failed is finished.
    IF TG_TABLE_NAME = 'tasks_execution' AND to_jsonb(NEW) ->> 'type' = 'RESET_TENANT' THEN
        RETURN NEW;
    END IF;
    IF EXISTS (
        SELECT 1 FROM sequent_backend.tenant
        WHERE id = NEW.tenant_id AND lifecycle_state = 'RESETTING'
    ) AND NOT sequent_backend.tenant_reset_allowed(NEW.tenant_id) THEN
        RAISE EXCEPTION 'tenant % is being reset', NEW.tenant_id
            USING ERRCODE = 'object_not_in_prerequisite_state';
    END IF;
    RETURN NEW;
END;
$$;

DO $$
DECLARE
    tbl text;
BEGIN
    FOR tbl IN
        SELECT c.relname
        FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        JOIN pg_attribute a ON a.attrelid = c.oid AND a.attname = 'tenant_id'
            AND NOT a.attisdropped
        WHERE n.nspname = 'sequent_backend' AND c.relkind IN ('r', 'p')
            AND c.relname NOT IN ('tenant', 'tenant_reset_baseline')
        ORDER BY c.relname
    LOOP
        EXECUTE format(
            'CREATE TRIGGER tenant_reset_refuse_insert BEFORE INSERT ON sequent_backend.%I '
            'FOR EACH ROW EXECUTE FUNCTION sequent_backend.tenant_reset_refuse_insert()',
            tbl
        );
    END LOOP;
END;
$$;

-- Guards split in two: the same trigger for INSERT/UPDATE, and a DELETE
-- trigger skipped for the tenant being reset in this transaction.
DROP TRIGGER ballot_box_seal_permanence ON sequent_backend.ballot_box_seal;
CREATE TRIGGER ballot_box_seal_permanence BEFORE INSERT OR UPDATE ON sequent_backend.ballot_box_seal
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.ballot_box_seal_permanence();
CREATE TRIGGER ballot_box_seal_permanence_delete BEFORE DELETE ON sequent_backend.ballot_box_seal
    FOR EACH ROW WHEN (NOT sequent_backend.tenant_reset_allowed(OLD.tenant_id))
    EXECUTE FUNCTION sequent_backend.ballot_box_seal_permanence();

DROP TRIGGER guard_publication_style_material ON sequent_backend.ballot_style;
CREATE TRIGGER guard_publication_style_material BEFORE INSERT OR UPDATE ON sequent_backend.ballot_style
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.guard_publication_style_material();
CREATE TRIGGER guard_publication_style_material_delete BEFORE DELETE ON sequent_backend.ballot_style
    FOR EACH ROW WHEN (NOT sequent_backend.tenant_reset_allowed(OLD.tenant_id))
    EXECUTE FUNCTION sequent_backend.guard_publication_style_material();

DROP TRIGGER cast_vote_seal_key_guard ON sequent_backend.cast_vote_seal_key;
CREATE TRIGGER cast_vote_seal_key_guard BEFORE UPDATE ON sequent_backend.cast_vote_seal_key
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.cast_vote_seal_guard();
CREATE TRIGGER cast_vote_seal_key_guard_delete BEFORE DELETE ON sequent_backend.cast_vote_seal_key
    FOR EACH ROW WHEN (NOT sequent_backend.tenant_reset_allowed(OLD.tenant_id))
    EXECUTE FUNCTION sequent_backend.cast_vote_seal_guard();

DROP TRIGGER cast_vote_sealing_required_guard ON sequent_backend.cast_vote_sealing_required;
CREATE TRIGGER cast_vote_sealing_required_guard BEFORE UPDATE ON sequent_backend.cast_vote_sealing_required
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.cast_vote_seal_guard();
CREATE TRIGGER cast_vote_sealing_required_guard_delete BEFORE DELETE ON sequent_backend.cast_vote_sealing_required
    FOR EACH ROW WHEN (NOT sequent_backend.tenant_reset_allowed(OLD.tenant_id))
    EXECUTE FUNCTION sequent_backend.cast_vote_seal_guard();

DROP TRIGGER ballot_box_seal_record_document_guard ON sequent_backend.document;
CREATE TRIGGER ballot_box_seal_record_document_guard BEFORE UPDATE ON sequent_backend.document
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.ballot_box_seal_record_document_guard();
CREATE TRIGGER ballot_box_seal_record_document_guard_delete BEFORE DELETE ON sequent_backend.document
    FOR EACH ROW WHEN (NOT sequent_backend.tenant_reset_allowed(OLD.tenant_id))
    EXECUTE FUNCTION sequent_backend.ballot_box_seal_record_document_guard();

DROP TRIGGER election_seal_guard ON sequent_backend.election;
CREATE TRIGGER election_seal_guard BEFORE DELETE ON sequent_backend.election
    FOR EACH ROW WHEN (NOT sequent_backend.tenant_reset_allowed(OLD.tenant_id))
    EXECUTE FUNCTION sequent_backend.election_seal_guard();

DROP TRIGGER guard_initialization_report_coverage ON sequent_backend.initialization_report_coverage;
CREATE TRIGGER guard_initialization_report_coverage BEFORE INSERT OR UPDATE ON sequent_backend.initialization_report_coverage
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.guard_initialization_report_coverage();
CREATE TRIGGER guard_initialization_report_coverage_delete BEFORE DELETE ON sequent_backend.initialization_report_coverage
    FOR EACH ROW WHEN (NOT sequent_backend.tenant_reset_allowed(OLD.tenant_id))
    EXECUTE FUNCTION sequent_backend.guard_initialization_report_coverage();

DROP TRIGGER monitoring_config_is_append_only ON sequent_backend.monitoring_config;
CREATE TRIGGER monitoring_config_is_append_only BEFORE UPDATE ON sequent_backend.monitoring_config
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_config_is_append_only();
CREATE TRIGGER monitoring_config_is_append_only_delete BEFORE DELETE ON sequent_backend.monitoring_config
    FOR EACH ROW WHEN (NOT sequent_backend.tenant_reset_allowed(OLD.tenant_id))
    EXECUTE FUNCTION sequent_backend.monitoring_config_is_append_only();

DROP TRIGGER monitoring_config_head_is_kept ON sequent_backend.monitoring_config_head;
CREATE TRIGGER monitoring_config_head_is_kept BEFORE DELETE ON sequent_backend.monitoring_config_head
    FOR EACH ROW WHEN (NOT sequent_backend.tenant_reset_allowed(OLD.tenant_id))
    EXECUTE FUNCTION sequent_backend.monitoring_deleted_with_event('monitoring_config_head_is_kept');

DROP TRIGGER monitoring_event_is_kept ON sequent_backend.monitoring_event;
CREATE TRIGGER monitoring_event_is_kept BEFORE DELETE ON sequent_backend.monitoring_event
    FOR EACH ROW WHEN (NOT sequent_backend.tenant_reset_allowed(OLD.tenant_id))
    EXECUTE FUNCTION sequent_backend.monitoring_event_is_kept();

DROP TRIGGER monitoring_snapshot_figure_is_kept ON sequent_backend.monitoring_snapshot_figure;
CREATE TRIGGER monitoring_snapshot_figure_is_kept BEFORE INSERT OR UPDATE ON sequent_backend.monitoring_snapshot_figure
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_snapshot_figure_is_kept();
CREATE TRIGGER monitoring_snapshot_figure_is_kept_delete BEFORE DELETE ON sequent_backend.monitoring_snapshot_figure
    FOR EACH ROW WHEN (NOT sequent_backend.tenant_reset_allowed(OLD.tenant_id))
    EXECUTE FUNCTION sequent_backend.monitoring_snapshot_figure_is_kept();

DROP TRIGGER monitoring_snapshot_payload_is_kept ON sequent_backend.monitoring_snapshot_payload;
CREATE TRIGGER monitoring_snapshot_payload_is_kept BEFORE DELETE ON sequent_backend.monitoring_snapshot_payload
    FOR EACH ROW WHEN (NOT sequent_backend.tenant_reset_allowed(OLD.tenant_id))
    EXECUTE FUNCTION sequent_backend.monitoring_snapshot_payload_is_kept();

DROP TRIGGER monitoring_snapshot_source_is_kept ON sequent_backend.monitoring_snapshot_source;
CREATE TRIGGER monitoring_snapshot_source_is_kept BEFORE INSERT ON sequent_backend.monitoring_snapshot_source
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_snapshot_source_is_kept();
CREATE TRIGGER monitoring_snapshot_source_is_kept_delete BEFORE DELETE ON sequent_backend.monitoring_snapshot_source
    FOR EACH ROW WHEN (NOT sequent_backend.tenant_reset_allowed(OLD.tenant_id))
    EXECUTE FUNCTION sequent_backend.monitoring_snapshot_source_is_kept();

DROP TRIGGER monitoring_snapshot_state_moves_forward ON sequent_backend.monitoring_snapshot_state;
CREATE TRIGGER monitoring_snapshot_state_moves_forward BEFORE UPDATE ON sequent_backend.monitoring_snapshot_state
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_snapshot_state_moves_forward();
CREATE TRIGGER monitoring_snapshot_state_moves_forward_delete BEFORE DELETE ON sequent_backend.monitoring_snapshot_state
    FOR EACH ROW WHEN (NOT sequent_backend.tenant_reset_allowed(OLD.tenant_id))
    EXECUTE FUNCTION sequent_backend.monitoring_snapshot_state_moves_forward();

DROP TRIGGER plugin_history_append_only ON sequent_backend.plugin_record_event;
CREATE TRIGGER plugin_history_append_only BEFORE UPDATE ON sequent_backend.plugin_record_event
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.plugin_history_append_only();
CREATE TRIGGER plugin_history_append_only_delete BEFORE DELETE ON sequent_backend.plugin_record_event
    FOR EACH ROW WHEN (NOT sequent_backend.tenant_reset_allowed(OLD.tenant_id))
    EXECUTE FUNCTION sequent_backend.plugin_history_append_only();

DROP TRIGGER guard_initialization_report_session ON sequent_backend.tally_session;
CREATE TRIGGER guard_initialization_report_session BEFORE UPDATE ON sequent_backend.tally_session
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.guard_initialization_report_session();
CREATE TRIGGER guard_initialization_report_session_delete BEFORE DELETE ON sequent_backend.tally_session
    FOR EACH ROW WHEN (NOT sequent_backend.tenant_reset_allowed(OLD.tenant_id))
    EXECUTE FUNCTION sequent_backend.guard_initialization_report_session();
