CREATE TABLE IF NOT EXISTS boards (
    id SERIAL UNIQUE,
    board_name VARCHAR PRIMARY KEY,
    created_at TIMESTAMP DEFAULT NOW(),
    is_archived BOOLEAN DEFAULT FALSE,
    status VARCHAR DEFAULT 'active',
    cfg_id VARCHAR,
    threshold_no INTEGER,
    trustees_no INTEGER,
    last_message_kind VARCHAR,
    last_updated TIMESTAMP,
    message_count INTEGER DEFAULT 0,
    batch_count INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS messages (
    id BIGSERIAL,
    board_name VARCHAR NOT NULL,
    timestamp BIGINT,
    size BIGINT,
    content_type VARCHAR,
    inline_data BYTEA,
    s3_key VARCHAR,
    created TIMESTAMP DEFAULT NOW(),
    statement_timestamp TIMESTAMP,
    message BYTEA,
    sender_pk VARCHAR NOT NULL,
    statement_kind VARCHAR NOT NULL,
    batch INTEGER NOT NULL DEFAULT 0,
    mix_number INTEGER NOT NULL DEFAULT 0,
    version VARCHAR NOT NULL,
    PRIMARY KEY (board_name, id),
    UNIQUE (board_name, sender_pk, statement_kind, batch, mix_number)
) PARTITION BY LIST (board_name);

CREATE INDEX IF NOT EXISTS idx_messages_board_id
ON messages(board_name, id);
