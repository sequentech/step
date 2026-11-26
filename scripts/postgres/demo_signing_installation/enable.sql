-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-- Opt-in offline demo provisioning, never run by migration/startup/import.
-- Requires the dedicated database owner, existing event and trusted install
-- run_id. No login, permissions, hardware credential or trust root is created.
\set ON_ERROR_STOP 1
BEGIN;
SELECT sequent_backend.operational_register_demo_scope(
    :'tenant'::uuid, :'event'::uuid, :'run_id'::uuid, :'backend'::text);
COMMIT;
