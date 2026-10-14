-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Prepared revisions have no document and are rebuilt from the dropped
-- columns, so they go too.
DELETE FROM sequent_backend.signing_document_revision WHERE document_id IS NULL;

DROP INDEX sequent_backend.signing_document_revision_field_signed_once;
DROP INDEX sequent_backend.signing_document_revision_one_base;

ALTER TABLE sequent_backend.signing_document_revision
    DROP CONSTRAINT signing_document_revision_stored,
    DROP CONSTRAINT signing_document_revision_digest_sha256_hex,
    DROP CONSTRAINT signing_document_revision_parent_sha256_hex,
    DROP CONSTRAINT signing_document_revision_field_index_counts,
    DROP COLUMN appearance,
    DROP COLUMN digest_sha256,
    DROP COLUMN parent_sha256,
    DROP COLUMN field_index,
    ALTER COLUMN document_id SET NOT NULL;
