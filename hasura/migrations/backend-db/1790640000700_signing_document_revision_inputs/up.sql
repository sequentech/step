-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- What a prepared PAdES revision is rebuilt from at approval: its parent's
-- SHA-256, the signature field it fills, the appearance it prints and the
-- ByteRange digest the signer was given. A prepared revision stores no
-- document (it is rebuilt, never trusted), so `document_id` may be empty.
ALTER TABLE sequent_backend.signing_document_revision
    ALTER COLUMN document_id DROP NOT NULL,
    ADD COLUMN field_index integer,
    ADD COLUMN parent_sha256 text,
    ADD COLUMN digest_sha256 text,
    ADD COLUMN appearance jsonb,
    ADD CONSTRAINT signing_document_revision_field_index_counts
        CHECK (field_index IS NULL OR field_index >= 0),
    ADD CONSTRAINT signing_document_revision_parent_sha256_hex
        CHECK (parent_sha256 IS NULL OR parent_sha256 ~ '^[0-9a-f]{64}$'),
    ADD CONSTRAINT signing_document_revision_digest_sha256_hex
        CHECK (digest_sha256 IS NULL OR digest_sha256 ~ '^[0-9a-f]{64}$'),
    -- Only a prepared revision has no stored document.
    ADD CONSTRAINT signing_document_revision_stored
        CHECK (state = 'prepared' OR document_id IS NOT NULL);

-- Backstops: one base per request, and one signature per field.
CREATE UNIQUE INDEX signing_document_revision_one_base
    ON sequent_backend.signing_document_revision (request_id)
    WHERE state = 'base';
CREATE UNIQUE INDEX signing_document_revision_field_signed_once
    ON sequent_backend.signing_document_revision (request_id, field_index)
    WHERE state = 'signed';
