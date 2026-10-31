-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- One row per country (area) of a Post each time the Post's initialization
-- report covers it: the dated record of the initialization, with the hash
-- printed on the report and the report document. A country is initialized
-- once it has a row; a new report adds rows, earlier ones stay as history.
-- A Post without areas gets one row with area_id NULL. A report records each
-- country once (a re-run of its tally adds nothing).
CREATE TABLE sequent_backend.election_initialization (
    id uuid NOT NULL DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    election_id uuid NOT NULL,
    area_id uuid,
    tally_session_id uuid NOT NULL,
    results_event_id uuid,
    report_hash text,
    document_id uuid,
    created_by text,
    created_by_username text,
    -- What the electoral log entry names (the Post's and the country's
    -- display names when the report completed).
    election_name text,
    area_name text,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    -- When its ElectionInitialized entry was staged in the signing log
    -- outbox (after the tally's commit, under the event's signing lock).
    log_staged_at timestamptz,
    PRIMARY KEY (id),
    CONSTRAINT election_initialization_of_its_post
        FOREIGN KEY (tenant_id, election_event_id, election_id)
        REFERENCES sequent_backend.election (tenant_id, election_event_id, id)
        ON DELETE CASCADE,
    CONSTRAINT election_initialization_of_its_area
        FOREIGN KEY (tenant_id, election_event_id, area_id)
        REFERENCES sequent_backend.area (tenant_id, election_event_id, id)
        ON DELETE CASCADE
);
CREATE INDEX election_initialization_by_post
    ON sequent_backend.election_initialization
    (tenant_id, election_event_id, election_id, area_id, created_at);
CREATE UNIQUE INDEX election_initialization_once_per_report
    ON sequent_backend.election_initialization
    (tally_session_id, election_id, COALESCE(area_id, '00000000-0000-0000-0000-000000000000'::uuid));
CREATE INDEX election_initialization_log_pending
    ON sequent_backend.election_initialization (tenant_id, election_event_id, created_at)
    WHERE log_staged_at IS NULL;
