-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Private issuance ledger, never tracked in Hasura. No credential, URL, voter
-- attributes, or plaintext delivery proof is stored. A signature authorizes an
-- exact initiation; it neither creates an enrollment nor verifies the voter.
CREATE TABLE sequent_backend.manual_verification_issuance (
    initiation_id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    requester_id text NOT NULL,
    voter_id text NOT NULL,
    post_id uuid,
    request_id uuid UNIQUE,
    authorized_at timestamptz,
    document_id uuid UNIQUE,
    secret_attributes text[] NOT NULL,
    render_started_at timestamptz,
    proof_sha256 bytea CHECK (octet_length(proof_sha256) = 32),
    minted_at timestamptz,
    ready_at timestamptz,
    redeemed_at timestamptz,
    failed_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    FOREIGN KEY (tenant_id,election_event_id)
        REFERENCES sequent_backend.election_event(tenant_id,id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id,election_event_id,request_id)
        REFERENCES sequent_backend.signing_request(tenant_id,election_event_id,id) ON DELETE CASCADE,
    CHECK (request_id IS NULL OR post_id IS NOT NULL),
    CHECK (request_id IS NOT NULL OR authorized_at IS NOT NULL),
    CHECK (document_id IS NULL OR authorized_at IS NOT NULL),
    CHECK ((render_started_at IS NULL) = (proof_sha256 IS NULL)),
    CHECK (minted_at IS NULL OR render_started_at IS NOT NULL),
    CHECK (ready_at IS NULL OR (minted_at IS NOT NULL AND failed_at IS NULL)),
    CHECK (redeemed_at IS NULL OR ready_at IS NOT NULL)
);
REVOKE ALL ON sequent_backend.manual_verification_issuance FROM PUBLIC;
