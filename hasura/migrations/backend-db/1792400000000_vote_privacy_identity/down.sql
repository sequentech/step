-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP TRIGGER check_ballot_privacy_identity ON sequent_backend.election_event;
DROP FUNCTION sequent_backend.check_ballot_privacy_identity();
-- Keep the recorded modes: removing them would reinterpret pseudonymous ballots.
