-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

ALTER TABLE sequent_backend.cast_vote
  ALTER COLUMN election_id DROP NOT NULL,
  ALTER COLUMN voter_id_string DROP NOT NULL,
  ALTER COLUMN ballot_id DROP NOT NULL;

CREATE INDEX IF NOT EXISTS cast_vote_participation_election_idx
ON sequent_backend.cast_vote (tenant_id, election_event_id, election_id, voter_id_string);

DROP INDEX IF EXISTS sequent_backend.cast_vote_voter_ballot_key;
