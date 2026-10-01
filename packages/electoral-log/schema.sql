-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

BEGIN;
CREATE TABLE IF NOT EXISTS electoral_log_boards (
    board_name TEXT PRIMARY KEY CHECK (board_name <> '')
);

CREATE TABLE IF NOT EXISTS electoral_log_messages (
    id BIGSERIAL PRIMARY KEY,
    board_name TEXT NOT NULL REFERENCES electoral_log_boards(board_name) ON DELETE CASCADE,
    delivery_id TEXT NOT NULL CHECK (delivery_id <> ''),
    created BIGINT NOT NULL,
    sender_pk TEXT NOT NULL,
    statement_timestamp BIGINT NOT NULL,
    statement_kind TEXT NOT NULL,
    message BYTEA NOT NULL,
    version TEXT NOT NULL,
    user_id TEXT,
    username TEXT,
    election_id TEXT,
    area_id TEXT,
    ballot_id TEXT,
    UNIQUE (board_name, delivery_id)
);
CREATE INDEX IF NOT EXISTS electoral_log_board_cursor ON electoral_log_messages (board_name, id);
CREATE INDEX IF NOT EXISTS electoral_log_cast_vote ON electoral_log_messages (board_name, statement_kind, election_id, id);
CREATE INDEX IF NOT EXISTS electoral_log_voter ON electoral_log_messages (board_name, user_id, ballot_id, id);
CREATE INDEX IF NOT EXISTS electoral_log_created ON electoral_log_messages (board_name, created, id);
COMMIT;
