-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

CREATE INDEX ballot_style_voter_reference_idx
    ON sequent_backend.ballot_style (tenant_id, election_event_id, area_id, election_id)
    INCLUDE (id, ballot_publication_id)
    WHERE deleted_at IS NULL;
