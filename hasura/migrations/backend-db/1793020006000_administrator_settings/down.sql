-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
DROP TRIGGER aaa_guard_administration_settings ON sequent_backend.election_event;
DROP FUNCTION sequent_backend.guard_administration_settings();
ALTER TABLE sequent_backend.election_event DROP COLUMN administration_settings;
ALTER TABLE sequent_backend.election DROP COLUMN administration_settings;
DROP FUNCTION sequent_backend.administration_settings(jsonb);
