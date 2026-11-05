-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP TRIGGER IF EXISTS guard_scheduler_fired_outcome ON sequent_backend.scheduled_event;
DROP FUNCTION IF EXISTS sequent_backend.guard_scheduler_fired_outcome();
