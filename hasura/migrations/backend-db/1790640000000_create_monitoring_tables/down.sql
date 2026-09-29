-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP TABLE IF EXISTS sequent_backend.monitoring_login_counter_receipt;
DROP TABLE IF EXISTS sequent_backend.monitoring_login_counter;
DROP TABLE IF EXISTS sequent_backend.monitoring_voter;
DROP TABLE IF EXISTS sequent_backend.monitoring_snapshot_payload;
DROP TABLE IF EXISTS sequent_backend.monitoring_snapshot_manifest;
ALTER TABLE IF EXISTS sequent_backend.monitoring_event
    DROP COLUMN IF EXISTS live_snapshot_revision;
DROP TABLE IF EXISTS sequent_backend.monitoring_snapshot_run;
DROP TABLE IF EXISTS sequent_backend.monitoring_config_head;
DROP TABLE IF EXISTS sequent_backend.monitoring_config;
DROP TABLE IF EXISTS sequent_backend.monitoring_event;
