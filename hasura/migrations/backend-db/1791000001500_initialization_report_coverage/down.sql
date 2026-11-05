-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
DROP TRIGGER guard_initialization_report_session ON sequent_backend.tally_session;
DROP FUNCTION sequent_backend.guard_initialization_report_session();
DROP TABLE sequent_backend.initialization_report_coverage;
DROP FUNCTION sequent_backend.guard_initialization_report_coverage();
