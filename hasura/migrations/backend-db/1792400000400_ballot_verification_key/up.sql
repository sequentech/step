-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

CREATE TABLE sequent_backend.ballot_verification_key (
    tenant_id UUID NOT NULL,
    election_event_id UUID NOT NULL,
    source_election_event_id UUID NOT NULL,
    board_log_uid UUID NOT NULL,
    board_name TEXT NOT NULL,
    ballot_box_key_id TEXT NOT NULL,
    system_public_key TEXT NOT NULL,
    ballot_box_public_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, election_event_id, board_log_uid, ballot_box_key_id),
    FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE
);
COMMENT ON TABLE sequent_backend.ballot_verification_key IS
    'Immutable public verification material retained with event evidence, independently of signing secrets and private mappings. Source identity survives archive imports.';
