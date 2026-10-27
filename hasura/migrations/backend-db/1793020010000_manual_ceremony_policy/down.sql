-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
DROP TRIGGER enforce_event_ceremony_policy_change ON sequent_backend.election_event;
DROP TRIGGER enforce_tally_event_ceremony_policy ON sequent_backend.tally_session;
DROP TRIGGER enforce_keys_event_ceremony_policy ON sequent_backend.keys_ceremony;
DROP FUNCTION sequent_backend.guard_event_ceremony_policy_change();
DROP FUNCTION sequent_backend.guard_ceremony_event_policy();
