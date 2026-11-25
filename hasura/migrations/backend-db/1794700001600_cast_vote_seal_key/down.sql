-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Run in a transaction. Rollback is allowed only while no event requires
-- sealing: dropping the tables would silently turn sealed events back into
-- plaintext ones, and lose the keys their stored ballots are sealed to. The
-- NOLOGIN group role is a cluster object other databases may use: it stays.
LOCK TABLE sequent_backend.cast_vote_sealing_required,
    sequent_backend.cast_vote_seal_key IN ACCESS EXCLUSIVE MODE;
DO $$
BEGIN
    IF EXISTS (SELECT FROM sequent_backend.cast_vote_sealing_required)
       OR EXISTS (SELECT FROM sequent_backend.cast_vote_seal_key) THEN
        RAISE EXCEPTION 'cast vote sealing is required by an election event; refuse rollback';
    END IF;
END;
$$;
DROP TRIGGER cast_vote_seal_voting_status_lock ON sequent_backend.election;
DROP TRIGGER cast_vote_seal_voting_status_lock ON sequent_backend.election_event;
DROP FUNCTION sequent_backend.cast_vote_seal_voting_status_lock();
DROP FUNCTION sequent_backend.cast_vote_seal_state(uuid, uuid);
DROP FUNCTION sequent_backend.cast_vote_seal_publish(uuid, uuid, text, integer, bytea);
DROP FUNCTION sequent_backend.cast_vote_seal_require(uuid, uuid);
DROP FUNCTION sequent_backend.cast_vote_seal_lock(uuid, uuid);
DROP FUNCTION sequent_backend.cast_vote_seal_voting_not_started(uuid, uuid);
DROP FUNCTION sequent_backend.cast_vote_seal_voting_started(jsonb);
DROP TABLE sequent_backend.cast_vote_seal_key;
DROP TABLE sequent_backend.cast_vote_sealing_required;
DROP FUNCTION sequent_backend.cast_vote_seal_guard();
REVOKE USAGE ON SCHEMA sequent_backend FROM sequent_os_cast_vote_key_publisher;
