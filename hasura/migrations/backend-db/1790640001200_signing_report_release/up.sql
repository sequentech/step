-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- How a report held for its signatures is released once its signing request
-- runs, and the e-mail it holds back.
--
-- target 'report': a report of the Reports tab, released under the document
-- id its generation answered (downloads wait for it).
-- target 'tally-result': a Post's (or a Post and country's) report of a
-- tally, released as that results row's PDF.
--
-- The release commits with the request's execution; the e-mail goes after the
-- commit, once (mail_started_at), and a failure is kept (mail_error), never
-- retried automatically.
CREATE TABLE sequent_backend.signing_report_release (
    request_id uuid NOT NULL,
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    target text NOT NULL,
    report_id uuid,
    document_id uuid,
    results_event_id uuid,
    election_id uuid,
    area_id uuid,
    report_type text,
    file_name text NOT NULL,
    is_public boolean NOT NULL DEFAULT false,
    encryption text NOT NULL DEFAULT 'none',
    email jsonb,
    released_at timestamptz,
    released_document_id uuid,
    released_name text,
    released_media_type text,
    mail_started_at timestamptz,
    mailed_at timestamptz,
    mail_error text,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (request_id),
    CONSTRAINT signing_report_release_target_known
        CHECK (target IN ('report', 'tally-result')),
    CONSTRAINT signing_report_release_encryption_known
        CHECK (encryption IN ('none', 'configured-password')),
    CONSTRAINT signing_report_release_report_document
        CHECK (target <> 'report' OR document_id IS NOT NULL),
    CONSTRAINT signing_report_release_tally_result
        CHECK (target <> 'tally-result'
            OR (results_event_id IS NOT NULL AND election_id IS NOT NULL
                AND report_type IS NOT NULL)),
    CONSTRAINT signing_report_release_of_its_request
        FOREIGN KEY (tenant_id, election_event_id, request_id)
        REFERENCES sequent_backend.signing_request (tenant_id, election_event_id, id)
        ON DELETE CASCADE
);

-- A tally's report held for its signatures, before its request starts. The
-- tally records it in its own transaction, which already holds locks on the
-- tally session (its executions reference it), so it takes no signing lock
-- there; the request starts after that commit, in a transaction that takes
-- the event's signing lock first (request_id, or error when it can't start).
CREATE TABLE sequent_backend.signing_tally_hold (
    id uuid NOT NULL DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    results_event_id uuid NOT NULL,
    action text NOT NULL,
    report_type text NOT NULL,
    election_id uuid NOT NULL,
    area_id uuid,
    file_name text NOT NULL,
    base_document_id uuid NOT NULL,
    base_sha256 text NOT NULL,
    -- Who ran the tally: the request's requester.
    requested_by text NOT NULL,
    requested_by_username text NOT NULL,
    request_id uuid,
    error text,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (id),
    CONSTRAINT signing_tally_hold_of_its_event
        FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE
);
CREATE INDEX signing_tally_hold_pending
    ON sequent_backend.signing_tally_hold (tenant_id, election_event_id, created_at)
    WHERE request_id IS NULL AND error IS NULL;
