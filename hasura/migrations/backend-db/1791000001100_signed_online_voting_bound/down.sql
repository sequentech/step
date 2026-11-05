-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP TRIGGER update_signed_voting_election ON sequent_backend.election;
DROP TRIGGER update_signed_voting_approval ON sequent_backend.signing_request;
DROP FUNCTION sequent_backend.update_signed_voting_election();
DROP FUNCTION sequent_backend.update_signed_voting_approval();
DROP FUNCTION sequent_backend.refresh_signed_voting_boundary(uuid, uuid);
DROP FUNCTION sequent_backend.signed_voting_channels(jsonb);
DROP FUNCTION sequent_backend.signed_voting_instant(text);
DROP INDEX sequent_backend.signing_request_executed_configuration_scope;
DROP TABLE sequent_backend.signed_voting_boundary;
DROP FUNCTION sequent_backend.live_voting_close_allowed(uuid, uuid, uuid);
DROP INDEX sequent_backend.lifecycle_snapshot_close_authority_scope;
ALTER TABLE sequent_backend.signing_request
    DROP COLUMN subject_close_required,
    DROP COLUMN subject_unsigned_close_policy,
    DROP COLUMN subject_publication_id;
ALTER TABLE sequent_backend.lifecycle_snapshot
    DROP COLUMN snapshot_close_required,
    DROP COLUMN snapshot_unsigned_close_policy;
