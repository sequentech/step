-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
ALTER TABLE sequent_backend.monitoring_voter DROP COLUMN has_voted;
DROP TRIGGER set_sequent_backend_monitoring_voter_updated_at ON sequent_backend.monitoring_voter;
CREATE TRIGGER set_sequent_backend_monitoring_voter_updated_at
BEFORE UPDATE ON sequent_backend.monitoring_voter FOR EACH ROW
EXECUTE FUNCTION sequent_backend.set_current_timestamp_updated_at();
DROP FUNCTION sequent_backend.monitoring_voter_activity_timestamp();
