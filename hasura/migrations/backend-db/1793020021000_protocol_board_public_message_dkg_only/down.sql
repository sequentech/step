-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

CREATE OR REPLACE FUNCTION sequent_backend.protocol_board_public_manager_message(
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
