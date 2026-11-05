-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP TRIGGER IF EXISTS guard_initialization_report_flag ON sequent_backend.election;
DROP FUNCTION IF EXISTS sequent_backend.guard_initialization_report_flag();
