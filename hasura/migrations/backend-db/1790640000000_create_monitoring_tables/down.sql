-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP TABLE IF EXISTS sequent_backend.monitoring_login_counter_receipt;
DROP TABLE IF EXISTS sequent_backend.monitoring_login_counter;
DROP TABLE IF EXISTS sequent_backend.monitoring_voter;
DROP TABLE IF EXISTS sequent_backend.monitoring_snapshot_figure;
DROP FUNCTION IF EXISTS sequent_backend.monitoring_snapshot_figure_is_kept();
DROP FUNCTION IF EXISTS sequent_backend.monitoring_snapshot_figure_completes_its_run();
DROP TABLE IF EXISTS sequent_backend.monitoring_snapshot_payload;
DROP TABLE IF EXISTS sequent_backend.monitoring_snapshot_source;
DROP TABLE IF EXISTS sequent_backend.monitoring_snapshot_state;
DROP FUNCTION IF EXISTS sequent_backend.monitoring_snapshot_state_moves_forward();
DROP TABLE IF EXISTS sequent_backend.monitoring_snapshot_run;
DROP FUNCTION IF EXISTS sequent_backend.monitoring_snapshot_run_is_final();
DROP SEQUENCE IF EXISTS sequent_backend.monitoring_snapshot_revision;
DROP TABLE IF EXISTS sequent_backend.monitoring_election_set;
DROP FUNCTION IF EXISTS sequent_backend.monitoring_uuids_ascend(uuid[]);
DROP TABLE IF EXISTS sequent_backend.monitoring_config_head;
DROP FUNCTION IF EXISTS sequent_backend.monitoring_config_head_moves_by_one();
DROP TABLE IF EXISTS sequent_backend.monitoring_config;
DROP FUNCTION IF EXISTS sequent_backend.monitoring_config_revision_is_the_head();
DROP FUNCTION IF EXISTS sequent_backend.monitoring_config_generation_is_current();
DROP FUNCTION IF EXISTS sequent_backend.monitoring_config_is_append_only();
DROP TABLE IF EXISTS sequent_backend.monitoring_event;
DROP FUNCTION IF EXISTS sequent_backend.monitoring_event_is_kept();
DROP FUNCTION IF EXISTS sequent_backend.monitoring_event_generation_moves_by_one();
DROP FUNCTION IF EXISTS sequent_backend.monitoring_deleted_with_event();
DROP FUNCTION IF EXISTS sequent_backend.monitoring_refuse_change();
-- btree_gist goes only if this migration created it and nothing else has
-- come to use it since.
DO $$
BEGIN
    IF obj_description(
        (SELECT oid FROM pg_extension WHERE extname = 'btree_gist'), 'pg_extension'
    ) = 'created by migration 1790640000000_create_monitoring_tables' THEN
        DROP EXTENSION btree_gist;
    END IF;
EXCEPTION WHEN dependent_objects_still_exist THEN
    RAISE NOTICE 'btree_gist is kept: other objects use it';
END;
$$;
