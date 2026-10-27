-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
DROP VIEW sequent_backend.election_lifecycle_access;
DROP TRIGGER seed_election_lifecycle ON sequent_backend.election_event;
DROP FUNCTION sequent_backend.seed_election_lifecycle();
DROP TABLE sequent_backend.lifecycle_notice;
DROP TABLE sequent_backend.lifecycle_service_lease;
DROP TABLE sequent_backend.election_lifecycle;
DROP FUNCTION sequent_backend.guard_election_lifecycle();

DROP FUNCTION IF EXISTS sequent_backend.audit_storage_allows_voters();
