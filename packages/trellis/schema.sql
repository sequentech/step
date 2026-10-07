-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
CREATE TABLE IF NOT EXISTS trellis_logs (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    size BIGINT NOT NULL DEFAULT 0 CHECK (size >= 0),
    root BYTEA NOT NULL DEFAULT '\xe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855' CHECK (octet_length(root) = 32)
);
-- Logs created before subtrees were stored gain the empty root here, which marks them
-- for a rebuild from their leaves before they are read or extended.
ALTER TABLE trellis_logs ADD COLUMN IF NOT EXISTS root BYTEA NOT NULL DEFAULT '\xe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855' CHECK (octet_length(root) = 32);
CREATE TABLE IF NOT EXISTS trellis_leaves (
    log_id BIGINT NOT NULL REFERENCES trellis_logs(id) ON DELETE CASCADE,
    leaf_index BIGINT NOT NULL CHECK (leaf_index >= 0),
    source_id BIGINT NOT NULL,
    hash BYTEA NOT NULL CHECK (octet_length(hash) = 32),
    PRIMARY KEY (log_id, leaf_index),
    UNIQUE (log_id, source_id)
);
-- Hash of the perfect subtree covering leaves [idx * 2^level, (idx + 1) * 2^level).
CREATE TABLE IF NOT EXISTS trellis_nodes (
    log_id BIGINT NOT NULL REFERENCES trellis_logs(id) ON DELETE CASCADE,
    level SMALLINT NOT NULL CHECK (level BETWEEN 1 AND 63),
    idx BIGINT NOT NULL CHECK (idx >= 0),
    hash BYTEA NOT NULL CHECK (octet_length(hash) = 32),
    PRIMARY KEY (log_id, level, idx)
);
