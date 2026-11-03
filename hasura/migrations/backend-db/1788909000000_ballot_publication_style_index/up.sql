-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Create the publication paging index within the migration transaction.
CREATE INDEX IF NOT EXISTS ballot_style_publication_page_idx
    ON sequent_backend.ballot_style (tenant_id, election_event_id, ballot_publication_id, id);
DO $$
BEGIN
    IF NOT (SELECT indisvalid FROM pg_index
            WHERE indexrelid = 'sequent_backend.ballot_style_publication_page_idx'::regclass) THEN
        RAISE EXCEPTION 'ballot_style_publication_page_idx is invalid; remove the invalid index before retrying this migration';
    END IF;
END;
$$;
