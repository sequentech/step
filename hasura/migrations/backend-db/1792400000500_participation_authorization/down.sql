-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
DROP TRIGGER protect_participation_document ON sequent_backend.document;
DROP FUNCTION sequent_backend.protect_participation_document();
DROP TRIGGER protect_participation_command ON sequent_backend.tasks_execution;
DROP TRIGGER protect_participation_command ON sequent_backend.scheduled_event;
DROP FUNCTION sequent_backend.protect_participation_command();
ALTER TABLE sequent_backend.tasks_execution DROP COLUMN participation_authorization;
ALTER TABLE sequent_backend.scheduled_event DROP COLUMN participation_authorization;
