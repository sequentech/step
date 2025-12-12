CREATE DATABASE b4;

\c b4

CREATE TABLE IF NOT EXISTS INDEX (
            id SERIAL PRIMARY KEY,
            board_name VARCHAR UNIQUE,
            is_archived BOOLEAN,
            cfg_id VARCHAR,
            threshold_no INT,
            trustees_no INT,
            last_message_kind VARCHAR,
            last_updated TIMESTAMP,
            message_count INT,
            batch_count INT DEFAULT 0,
            UNIQUE(board_name)
        );
