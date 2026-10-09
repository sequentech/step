-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

DROP TRIGGER IF EXISTS "guard_election_server_state"
ON "sequent_backend"."election";

DROP FUNCTION IF EXISTS "sequent_backend"."guard_election_server_state"();

DROP FUNCTION IF EXISTS "sequent_backend"."election_server_status"(jsonb);
