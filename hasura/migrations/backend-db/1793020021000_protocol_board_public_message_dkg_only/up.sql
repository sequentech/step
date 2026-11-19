-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- The computed field public_manager_message returns a dkg board's
-- Configuration only. A tally board's Ballots message, the encrypted votes of
-- its batch, is read in the verification data download, whose every export is
-- recorded; GraphQL roles no longer read it in post-processing.
CREATE OR REPLACE FUNCTION sequent_backend.protocol_board_public_manager_message(
    board sequent_backend.protocol_board
)
RETURNS bytea
LANGUAGE sql
STABLE
AS $$
    SELECT board.manager_message
    WHERE board.parent_id IS NULL;
$$;
