-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP TABLE IF EXISTS sequent_backend.monitoring_login_counter_receipt;
DROP TABLE IF EXISTS sequent_backend.monitoring_login_counter;
DROP TABLE IF EXISTS sequent_backend.monitoring_voter;
DROP TABLE IF EXISTS sequent_backend.monitoring_snapshot_source;
DROP TABLE IF EXISTS sequent_backend.monitoring_snapshot_manifest_scope;
DROP TABLE IF EXISTS sequent_backend.monitoring_snapshot_manifest;
DROP TABLE IF EXISTS sequent_backend.monitoring_snapshot_payload;
ALTER TABLE IF EXISTS sequent_backend.monitoring_event
    DROP CONSTRAINT IF EXISTS monitoring_event_shows_a_complete_run;
DROP TABLE IF EXISTS sequent_backend.monitoring_snapshot_run;
DROP SEQUENCE IF EXISTS sequent_backend.monitoring_snapshot_revision;
DROP TABLE IF EXISTS sequent_backend.monitoring_election_set;
DROP FUNCTION IF EXISTS sequent_backend.monitoring_uuids_ascend(uuid[]);
DROP TABLE IF EXISTS sequent_backend.monitoring_config_head;
DROP TABLE IF EXISTS sequent_backend.monitoring_config;
DROP FUNCTION IF EXISTS sequent_backend.monitoring_config_is_append_only();
DROP TABLE IF EXISTS sequent_backend.monitoring_event;
