-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

ALTER TABLE sequent_backend.scheduled_event
    DROP CONSTRAINT IF EXISTS scheduled_event_voting_period_valid;
DROP INDEX IF EXISTS sequent_backend.scheduled_event_active_voting_task_idx;
DROP INDEX IF EXISTS sequent_backend.scheduled_event_active_scope_task_idx;
