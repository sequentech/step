-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP TRIGGER IF EXISTS "guard_trusted_lockdown_insert"
ON "sequent_backend"."election_event";

DROP TRIGGER IF EXISTS "guard_trusted_lockdown_update"
ON "sequent_backend"."election_event";

DROP TRIGGER IF EXISTS "guard_trusted_voting_status_insert"
ON "sequent_backend"."election_event";

DROP TRIGGER IF EXISTS "guard_trusted_voting_status_update"
ON "sequent_backend"."election_event";

DROP TRIGGER IF EXISTS "guard_trusted_grace_period_update"
ON "sequent_backend"."election";

DROP TRIGGER IF EXISTS "guard_trusted_voting_status_insert"
ON "sequent_backend"."election";

DROP TRIGGER IF EXISTS "guard_trusted_voting_status_update"
ON "sequent_backend"."election";

DROP FUNCTION IF EXISTS "sequent_backend"."guard_trusted_grace_period_update"();
DROP FUNCTION IF EXISTS "sequent_backend"."guard_trusted_lockdown_update"();
DROP FUNCTION IF EXISTS "sequent_backend"."guard_trusted_voting_status_update"();
DROP FUNCTION IF EXISTS "sequent_backend"."trusted_grace_period"(jsonb);
DROP FUNCTION IF EXISTS "sequent_backend"."trusted_voting_started"(jsonb);
DROP FUNCTION IF EXISTS "sequent_backend"."trusted_write_allowed"();
DROP FUNCTION IF EXISTS "sequent_backend"."trusted_voting_state"(jsonb);
DROP FUNCTION IF EXISTS "sequent_backend"."trusted_write_instant"(jsonb);
