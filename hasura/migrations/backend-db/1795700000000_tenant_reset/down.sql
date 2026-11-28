-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM sequent_backend.tenant_reset_state) THEN
        RAISE EXCEPTION 'cannot downgrade while a tenant reset is unfinished';
    END IF;
END;
$$;

DROP TRIGGER ballot_box_seal_permanence_delete ON sequent_backend.ballot_box_seal;
DROP TRIGGER ballot_box_seal_permanence ON sequent_backend.ballot_box_seal;
CREATE TRIGGER ballot_box_seal_permanence BEFORE INSERT OR DELETE OR UPDATE ON sequent_backend.ballot_box_seal
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.ballot_box_seal_permanence();

DROP TRIGGER guard_publication_style_material_delete ON sequent_backend.ballot_style;
DROP TRIGGER guard_publication_style_material ON sequent_backend.ballot_style;
CREATE TRIGGER guard_publication_style_material BEFORE INSERT OR DELETE OR UPDATE ON sequent_backend.ballot_style
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.guard_publication_style_material();

DROP TRIGGER ballot_box_seal_record_document_guard_delete ON sequent_backend.document;
DROP TRIGGER ballot_box_seal_record_document_guard ON sequent_backend.document;
CREATE TRIGGER ballot_box_seal_record_document_guard BEFORE DELETE OR UPDATE ON sequent_backend.document
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.ballot_box_seal_record_document_guard();

DROP TRIGGER election_seal_guard ON sequent_backend.election;
CREATE TRIGGER election_seal_guard BEFORE DELETE ON sequent_backend.election
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.election_seal_guard();

DROP TRIGGER guard_initialization_report_coverage_delete ON sequent_backend.initialization_report_coverage;
DROP TRIGGER guard_initialization_report_coverage ON sequent_backend.initialization_report_coverage;
CREATE TRIGGER guard_initialization_report_coverage BEFORE INSERT OR DELETE OR UPDATE ON sequent_backend.initialization_report_coverage
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.guard_initialization_report_coverage();

DROP TRIGGER monitoring_config_is_append_only_delete ON sequent_backend.monitoring_config;
DROP TRIGGER monitoring_config_is_append_only ON sequent_backend.monitoring_config;
CREATE TRIGGER monitoring_config_is_append_only BEFORE DELETE OR UPDATE ON sequent_backend.monitoring_config
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_config_is_append_only();

DROP TRIGGER monitoring_config_head_is_kept ON sequent_backend.monitoring_config_head;
CREATE TRIGGER monitoring_config_head_is_kept BEFORE DELETE ON sequent_backend.monitoring_config_head
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_deleted_with_event('monitoring_config_head_is_kept');

DROP TRIGGER monitoring_event_is_kept ON sequent_backend.monitoring_event;
CREATE TRIGGER monitoring_event_is_kept BEFORE DELETE ON sequent_backend.monitoring_event
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_event_is_kept();

DROP TRIGGER monitoring_snapshot_figure_is_kept_delete ON sequent_backend.monitoring_snapshot_figure;
DROP TRIGGER monitoring_snapshot_figure_is_kept ON sequent_backend.monitoring_snapshot_figure;
CREATE TRIGGER monitoring_snapshot_figure_is_kept BEFORE INSERT OR DELETE OR UPDATE ON sequent_backend.monitoring_snapshot_figure
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_snapshot_figure_is_kept();

DROP TRIGGER monitoring_snapshot_payload_is_kept ON sequent_backend.monitoring_snapshot_payload;
CREATE TRIGGER monitoring_snapshot_payload_is_kept BEFORE DELETE ON sequent_backend.monitoring_snapshot_payload
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_snapshot_payload_is_kept();

DROP TRIGGER monitoring_snapshot_source_is_kept_delete ON sequent_backend.monitoring_snapshot_source;
DROP TRIGGER monitoring_snapshot_source_is_kept ON sequent_backend.monitoring_snapshot_source;
CREATE TRIGGER monitoring_snapshot_source_is_kept BEFORE INSERT OR DELETE ON sequent_backend.monitoring_snapshot_source
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_snapshot_source_is_kept();

DROP TRIGGER monitoring_snapshot_state_moves_forward_delete ON sequent_backend.monitoring_snapshot_state;
DROP TRIGGER monitoring_snapshot_state_moves_forward ON sequent_backend.monitoring_snapshot_state;
CREATE TRIGGER monitoring_snapshot_state_moves_forward BEFORE DELETE OR UPDATE ON sequent_backend.monitoring_snapshot_state
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.monitoring_snapshot_state_moves_forward();

DROP TRIGGER plugin_history_append_only_delete ON sequent_backend.plugin_record_event;
DROP TRIGGER plugin_history_append_only ON sequent_backend.plugin_record_event;
CREATE TRIGGER plugin_history_append_only BEFORE DELETE OR UPDATE ON sequent_backend.plugin_record_event
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.plugin_history_append_only();

DROP TRIGGER guard_initialization_report_session_delete ON sequent_backend.tally_session;
DROP TRIGGER guard_initialization_report_session ON sequent_backend.tally_session;
CREATE TRIGGER guard_initialization_report_session BEFORE DELETE OR UPDATE ON sequent_backend.tally_session
    FOR EACH ROW EXECUTE FUNCTION sequent_backend.guard_initialization_report_session();

DO $$
DECLARE
    tbl text;
BEGIN
    FOR tbl IN
        SELECT c.relname
        FROM pg_trigger t
        JOIN pg_class c ON c.oid = t.tgrelid
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = 'sequent_backend' AND t.tgname = 'tenant_reset_refuse_insert'
    LOOP
        EXECUTE format('DROP TRIGGER tenant_reset_refuse_insert ON sequent_backend.%I', tbl);
    END LOOP;
END;
$$;

DROP TRIGGER tenant_reset_refuse_new_tenant ON sequent_backend.tenant;
DROP FUNCTION sequent_backend.tenant_reset_refuse_new_tenant();
DROP FUNCTION sequent_backend.tenant_reset_refuse_insert();
DROP FUNCTION sequent_backend.tenant_reset_allowed(uuid);
DROP FUNCTION sequent_backend.tenant_resetting(uuid);
DROP TABLE sequent_backend.tenant_reset_baseline;
DROP TABLE sequent_backend.tenant_reset_state;
