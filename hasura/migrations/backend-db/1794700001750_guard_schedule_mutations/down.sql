-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP TRIGGER guard_hasura_schedule_mutation ON sequent_backend.scheduled_event;
DROP FUNCTION sequent_backend.guard_hasura_schedule_mutation();
