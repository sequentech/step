-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- The manager message of a board as GraphQL roles read it, through the
-- computed field public_manager_message of sequent_backend_protocol_board.
-- A dkg board's Configuration is returned as stored. A tally board's Ballots
-- message carries the encrypted votes of its batch, so it is returned only
-- while the election event is in post-processing, the phase in which
-- verification data are read; in every other phase the result is NULL.
CREATE FUNCTION sequent_backend.protocol_board_public_manager_message(
    board sequent_backend.protocol_board
)
RETURNS bytea
LANGUAGE sql
STABLE
AS $$
    SELECT board.manager_message
    WHERE board.parent_id IS NULL
       OR EXISTS (
           SELECT
           FROM sequent_backend.election_lifecycle lifecycle
           WHERE lifecycle.tenant_id = board.tenant_id
             AND lifecycle.election_event_id = board.election_event_id
             AND lifecycle.phase = 'post_processing'
       );
$$;
