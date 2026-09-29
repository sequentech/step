-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP TABLE IF EXISTS sequent_backend.monitoring_login_counter_receipt;
DROP TABLE IF EXISTS sequent_backend.monitoring_login_counter;
DROP TABLE IF EXISTS sequent_backend.monitoring_voter;
DROP TABLE IF EXISTS sequent_backend.monitoring_snapshot_figure;
DROP FUNCTION IF EXISTS sequent_backend.monitoring_snapshot_figure_is_kept();
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
DROP FUNCTION IF EXISTS sequent_backend.monitoring_refuse_change();
DROP EXTENSION IF EXISTS btree_gist;
