-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP TRIGGER guard_election_lifecycle_identity ON sequent_backend.election;
DROP FUNCTION sequent_backend.guard_election_lifecycle_identity();
DROP TRIGGER update_signed_voting_election ON sequent_backend.election;
CREATE TRIGGER update_signed_voting_election
AFTER INSERT OR UPDATE OF tenant_id, election_event_id OR DELETE
ON sequent_backend.election
FOR EACH ROW EXECUTE FUNCTION sequent_backend.update_signed_voting_election();
