-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP TRIGGER check_election_privacy_channels ON sequent_backend.election;
DROP FUNCTION sequent_backend.check_election_privacy_channels();
DROP TRIGGER check_event_privacy_channels ON sequent_backend.election_event;
DROP FUNCTION sequent_backend.check_event_privacy_channels();
