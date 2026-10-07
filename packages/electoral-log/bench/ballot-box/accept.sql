-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

-- pgbench script: accept one vote in one statement. The voter-state upsert
-- enforces the revote limit; a vote over the limit inserts nothing. run.sh
-- replaces @BYTES@ with the ballot size, so that the constant ballot content is
-- built once per prepared statement rather than per vote.
-- Variables: voters (distinct voters), elections, max_votes.
\set voter random(1, :voters)
\set election random(1, :elections)
WITH state AS (
    INSERT INTO voter_state AS s
        (election_event_id, election_id, voter, votes, last_ballot_id, updated_at)
    VALUES (
        '00000000-0000-4000-8000-000000000001',
        ('00000000-0000-4000-8000-' || lpad(:election::text, 12, '0'))::uuid,
        sha256(('voter-' || :voter)::bytea),
        1,
        sha256((:client_id || '-' || clock_timestamp()::text || random()::text)::bytea),
        now()
    )
    ON CONFLICT (election_event_id, election_id, voter) DO UPDATE
        SET votes = s.votes + 1,
            last_ballot_id = EXCLUDED.last_ballot_id,
            updated_at = EXCLUDED.updated_at
        WHERE s.votes < :max_votes
    RETURNING last_ballot_id
)
INSERT INTO ballot (election_event_id, ballot_id, election_id, voter, area_id, content)
SELECT '00000000-0000-4000-8000-000000000001',
       state.last_ballot_id,
       ('00000000-0000-4000-8000-' || lpad(:election::text, 12, '0'))::uuid,
       sha256(('voter-' || :voter)::bytea),
       NULL,
       bench_content(@BYTES@)
FROM state;
