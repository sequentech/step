-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP TRIGGER clear_election_voting_windows ON sequent_backend.scheduled_event;
DROP TRIGGER update_election_voting_window ON sequent_backend.scheduled_event;
DROP FUNCTION sequent_backend.clear_election_voting_windows();
DROP FUNCTION sequent_backend.update_election_voting_window();
DROP FUNCTION sequent_backend.refresh_election_voting_window(uuid, uuid, uuid);
DROP FUNCTION sequent_backend.voting_window_election_id(sequent_backend.scheduled_event);
DROP TABLE sequent_backend.election_voting_window;
DROP INDEX sequent_backend.scheduled_event_active_scope_task_idx;
