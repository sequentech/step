-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

ALTER TABLE sequent_backend.scheduled_event DROP CONSTRAINT scheduled_event_voting_period_valid;

-- Without the channel field, rolled-back schedules fall back to the legacy
-- ONLINE + KIOSK behavior. The preceding schema has no payload constraint.
UPDATE sequent_backend.scheduled_event
SET event_payload = event_payload - 'voting_channels'
WHERE event_payload ? 'voting_channels';
