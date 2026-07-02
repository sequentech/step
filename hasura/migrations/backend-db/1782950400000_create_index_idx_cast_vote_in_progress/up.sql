CREATE INDEX IF NOT EXISTS idx_cast_vote_in_progress
ON sequent_backend.cast_vote (election_id, voter_id_string, created_at DESC)
WHERE status = 'in-progress';
