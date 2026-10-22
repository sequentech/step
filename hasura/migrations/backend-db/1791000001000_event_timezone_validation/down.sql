-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Canonical stored zone names remain valid when rolling this guard back.
DROP TRIGGER validate_election_timezone ON sequent_backend.election;
DROP TRIGGER validate_event_timezones ON sequent_backend.election_event;
DROP FUNCTION sequent_backend.validate_election_timezone();
DROP FUNCTION sequent_backend.validate_event_timezones();
DROP FUNCTION sequent_backend.canonical_event_timezone(text);
