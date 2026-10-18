-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Checksum of the metadata and migrations baked into the Hasura image that
-- were last applied to this database. The image entrypoint skips the apply on
-- start when it matches, so restarted pods don't bump the metadata resource
-- version and make every replica rebuild its schema cache. Not tracked in
-- Hasura metadata.
CREATE SCHEMA IF NOT EXISTS sequent_ops;

CREATE TABLE IF NOT EXISTS sequent_ops.hasura_apply_state (
    id integer PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    checksum text NOT NULL,
    applied_at timestamptz NOT NULL DEFAULT now()
);
