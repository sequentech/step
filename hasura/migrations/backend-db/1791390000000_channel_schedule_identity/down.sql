-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

ALTER TABLE sequent_backend.scheduled_event
    DROP CONSTRAINT scheduled_event_channel_task_valid;
