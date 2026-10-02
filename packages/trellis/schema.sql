-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
CREATE TABLE IF NOT EXISTS trellis_logs (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    size BIGINT NOT NULL DEFAULT 0 CHECK (size >= 0)
);
CREATE TABLE IF NOT EXISTS trellis_leaves (
    log_id BIGINT NOT NULL REFERENCES trellis_logs(id) ON DELETE CASCADE,
    leaf_index BIGINT NOT NULL CHECK (leaf_index >= 0),
    source_id BIGINT NOT NULL,
    hash BYTEA NOT NULL CHECK (octet_length(hash) = 32),
    PRIMARY KEY (log_id, leaf_index),
    UNIQUE (log_id, source_id)
);
