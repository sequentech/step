-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Internal generation input; kept out of the publication annotations read by voters.
CREATE TABLE sequent_backend.ballot_publication_snapshot (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    ballot_publication_id uuid NOT NULL,
    generation_id uuid NOT NULL,
    snapshot jsonb NOT NULL,
    PRIMARY KEY (tenant_id, election_event_id, ballot_publication_id),
    FOREIGN KEY (ballot_publication_id, tenant_id, election_event_id)
        REFERENCES sequent_backend.ballot_publication (id, tenant_id, election_event_id)
        ON DELETE CASCADE
);

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
