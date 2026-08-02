CREATE INDEX IF NOT EXISTS cast_vote_participation_event_idx
ON sequent_backend.cast_vote (tenant_id, election_event_id, voter_id_string);

CREATE INDEX IF NOT EXISTS cast_vote_participation_election_idx
ON sequent_backend.cast_vote (tenant_id, election_event_id, election_id, voter_id_string);
