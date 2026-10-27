CREATE OR REPLACE VIEW sequent_backend.election_lifecycle_access AS
 SELECT tenant_id,election_event_id,phase,safety,generation,false AS voter_allowed
 FROM sequent_backend.election_lifecycle;
-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
DROP TABLE sequent_backend.self_test_run;
DROP TABLE sequent_backend.self_test_baseline;
DROP TABLE sequent_backend.self_test_settings;
